/*++

FuckScanDrive Minifilter Driver

This driver intercepts file system operations at the kernel level
and blocks access to specified drives for specific processes.

--*/

#include <fltKernel.h>
#include <dontuse.h>
#include <suppress.h>

#pragma prefast(disable:__WARNING_ENCODE_MEMBER_FUNCTION_POINTER, "Not valid for kernel mode drivers")

//
// Global Data
//

#define FUCKSCAN_PORT_NAME L"\\FuckScanPort"
#define MAX_BLOCKED_PROCESSES 100
#define MAX_PROCESS_NAME_LEN 256

typedef struct _FUCKSCAN_DATA {
    PFLT_FILTER FilterHandle;
    PFLT_PORT ServerPort;
    PFLT_PORT ClientPort;
} FUCKSCAN_DATA, *PFUCKSCAN_DATA;

typedef struct _BLOCKED_PROCESS_RULE {
    WCHAR ProcessName[MAX_PROCESS_NAME_LEN];
    BOOLEAN BlockAllDrives;
    WCHAR BlockedDrives[26];  // A-Z
    ULONG BlockedDriveCount;
} BLOCKED_PROCESS_RULE, *PBLOCKED_PROCESS_RULE;

typedef struct _FUCKSCAN_CONTEXT {
    BLOCKED_PROCESS_RULE Rules[MAX_BLOCKED_PROCESSES];
    ULONG RuleCount;
    ERESOURCE RulesLock;
} FUCKSCAN_CONTEXT, *PFUCKSCAN_CONTEXT;

FUCKSCAN_DATA g_FuckScanData;
FUCKSCAN_CONTEXT g_Context;

//
// Function Prototypes
//

DRIVER_INITIALIZE DriverEntry;
NTSTATUS FuckScanInstanceSetup(_In_ PCFLT_RELATED_OBJECTS FltObjects, _In_ FLT_INSTANCE_SETUP_FLAGS Flags, _In_ DEVICE_TYPE VolumeDeviceType, _In_ FLT_FILESYSTEM_TYPE VolumeFilesystemType);
VOID FuckScanInstanceTeardownStart(_In_ PCFLT_RELATED_OBJECTS FltObjects, _In_ FLT_INSTANCE_TEARDOWN_FLAGS Flags);
VOID FuckScanInstanceTeardownComplete(_In_ PCFLT_RELATED_OBJECTS FltObjects, _In_ FLT_INSTANCE_TEARDOWN_FLAGS Flags);
NTSTATUS FuckScanUnload(_In_ FLT_FILTER_UNLOAD_FLAGS Flags);
FLT_PREOP_CALLBACK_STATUS FuckScanPreCreate(_Inout_ PFLT_CALLBACK_DATA Data, _In_ PCFLT_RELATED_OBJECTS FltObjects, _Flt_CompletionContext_Outptr_ PVOID *CompletionContext);
NTSTATUS FuckScanPortConnect(_In_ PFLT_PORT ClientPort, _In_opt_ PVOID ServerPortCookie, _In_reads_bytes_opt_(SizeOfContext) PVOID ConnectionContext, _In_ ULONG SizeOfContext, _Outptr_result_maybenull_ PVOID *ConnectionCookie);
VOID FuckScanPortDisconnect(_In_opt_ PVOID ConnectionCookie);
NTSTATUS FuckScanPortMessage(_In_ PVOID ConnectionCookie, _In_reads_bytes_opt_(InputBufferSize) PVOID InputBuffer, _In_ ULONG InputBufferSize, _Out_writes_bytes_to_opt_(OutputBufferSize,*ReturnOutputBufferLength) PVOID OutputBuffer, _In_ ULONG OutputBufferSize, _Out_ PULONG ReturnOutputBufferLength);

//
// Minifilter Registration
//

CONST FLT_OPERATION_REGISTRATION Callbacks[] = {
    { IRP_MJ_CREATE, 0, FuckScanPreCreate, NULL },
    { IRP_MJ_OPERATION_END }
};

CONST FLT_REGISTRATION FilterRegistration = {
    sizeof(FLT_REGISTRATION),
    FLT_REGISTRATION_VERSION,
    0,
    NULL,
    Callbacks,
    FuckScanUnload,
    FuckScanInstanceSetup,
    NULL,
    FuckScanInstanceTeardownStart,
    FuckScanInstanceTeardownComplete,
    NULL,
    NULL,
    NULL,
    NULL
};

//
// Helper Functions
//

BOOLEAN IsProcessBlocked(_In_ PUNICODE_STRING ProcessName, _In_ WCHAR DriveLetter)
{
    BOOLEAN blocked = FALSE;
    ULONG i;

    ExEnterCriticalRegionAndAcquireResourceShared(&g_Context.RulesLock);

    for (i = 0; i < g_Context.RuleCount; i++) {
        if (wcsstr(ProcessName->Buffer, g_Context.Rules[i].ProcessName) != NULL) {
            if (g_Context.Rules[i].BlockAllDrives) {
                blocked = TRUE;
                break;
            }

            for (ULONG j = 0; j < g_Context.Rules[i].BlockedDriveCount; j++) {
                if (RtlUpcaseUnicodeChar(DriveLetter) == RtlUpcaseUnicodeChar(g_Context.Rules[i].BlockedDrives[j])) {
                    blocked = TRUE;
                    break;
                }
            }

            if (blocked) break;
        }
    }

    ExReleaseResourceAndLeaveCriticalRegion(&g_Context.RulesLock);

    return blocked;
}

WCHAR GetDriveLetter(_In_ PUNICODE_STRING FileName)
{
    if (FileName->Length >= 2 * sizeof(WCHAR)) {
        if (FileName->Buffer[1] == L':') {
            return FileName->Buffer[0];
        }
    }

    if (FileName->Length >= 6 * sizeof(WCHAR)) {
        if (wcsncmp(FileName->Buffer, L"\\??\\", 4) == 0 && FileName->Buffer[5] == L':') {
            return FileName->Buffer[4];
        }
    }

    return 0;
}

//
// Minifilter Callbacks
//

FLT_PREOP_CALLBACK_STATUS FuckScanPreCreate(_Inout_ PFLT_CALLBACK_DATA Data, _In_ PCFLT_RELATED_OBJECTS FltObjects, _Flt_CompletionContext_Outptr_ PVOID *CompletionContext)
{
    NTSTATUS status;
    PEPROCESS process;
    PUNICODE_STRING processName;
    WCHAR driveLetter;

    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(CompletionContext);

    if (g_Context.RuleCount == 0) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    process = IoThreadToProcess(Data->Thread);
    if (process == NULL) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    status = SeLocateProcessImageName(process, &processName);
    if (!NT_SUCCESS(status)) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    driveLetter = GetDriveLetter(&Data->Iopb->TargetFileObject->FileName);
    if (driveLetter == 0) {
        ExFreePool(processName);
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    if (IsProcessBlocked(processName, driveLetter)) {
        ExFreePool(processName);
        Data->IoStatus.Status = STATUS_ACCESS_DENIED;
        Data->IoStatus.Information = 0;
        return FLT_PREOP_COMPLETE;
    }

    ExFreePool(processName);
    return FLT_PREOP_SUCCESS_NO_CALLBACK;
}

//
// Port Communication
//

NTSTATUS FuckScanPortConnect(_In_ PFLT_PORT ClientPort, _In_opt_ PVOID ServerPortCookie, _In_reads_bytes_opt_(SizeOfContext) PVOID ConnectionContext, _In_ ULONG SizeOfContext, _Outptr_result_maybenull_ PVOID *ConnectionCookie)
{
    UNREFERENCED_PARAMETER(ServerPortCookie);
    UNREFERENCED_PARAMETER(ConnectionContext);
    UNREFERENCED_PARAMETER(SizeOfContext);
    UNREFERENCED_PARAMETER(ConnectionCookie);

    g_FuckScanData.ClientPort = ClientPort;
    return STATUS_SUCCESS;
}

VOID FuckScanPortDisconnect(_In_opt_ PVOID ConnectionCookie)
{
    UNREFERENCED_PARAMETER(ConnectionCookie);

    FltCloseClientPort(g_FuckScanData.FilterHandle, &g_FuckScanData.ClientPort);
    g_FuckScanData.ClientPort = NULL;
}

NTSTATUS FuckScanPortMessage(_In_ PVOID ConnectionCookie, _In_reads_bytes_opt_(InputBufferSize) PVOID InputBuffer, _In_ ULONG InputBufferSize, _Out_writes_bytes_to_opt_(OutputBufferSize,*ReturnOutputBufferLength) PVOID OutputBuffer, _In_ ULONG OutputBufferSize, _Out_ PULONG ReturnOutputBufferLength)
{
    UNREFERENCED_PARAMETER(ConnectionCookie);
    UNREFERENCED_PARAMETER(OutputBuffer);
    UNREFERENCED_PARAMETER(OutputBufferSize);

    *ReturnOutputBufferLength = 0;

    if (InputBuffer == NULL || InputBufferSize < sizeof(BLOCKED_PROCESS_RULE)) {
        return STATUS_INVALID_PARAMETER;
    }

    ExEnterCriticalRegionAndAcquireResourceExclusive(&g_Context.RulesLock);

    RtlCopyMemory(&g_Context.Rules[g_Context.RuleCount], InputBuffer, sizeof(BLOCKED_PROCESS_RULE));
    g_Context.RuleCount++;

    if (g_Context.RuleCount >= MAX_BLOCKED_PROCESSES) {
        g_Context.RuleCount = MAX_BLOCKED_PROCESSES;
    }

    ExReleaseResourceAndLeaveCriticalRegion(&g_Context.RulesLock);

    return STATUS_SUCCESS;
}

//
// Instance Setup/Teardown
//

NTSTATUS FuckScanInstanceSetup(_In_ PCFLT_RELATED_OBJECTS FltObjects, _In_ FLT_INSTANCE_SETUP_FLAGS Flags, _In_ DEVICE_TYPE VolumeDeviceType, _In_ FLT_FILESYSTEM_TYPE VolumeFilesystemType)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);
    UNREFERENCED_PARAMETER(VolumeDeviceType);
    UNREFERENCED_PARAMETER(VolumeFilesystemType);

    return STATUS_SUCCESS;
}

VOID FuckScanInstanceTeardownStart(_In_ PCFLT_RELATED_OBJECTS FltObjects, _In_ FLT_INSTANCE_TEARDOWN_FLAGS Flags)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);
}

VOID FuckScanInstanceTeardownComplete(_In_ PCFLT_RELATED_OBJECTS FltObjects, _In_ FLT_INSTANCE_TEARDOWN_FLAGS Flags)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);
}

//
// Driver Unload
//

NTSTATUS FuckScanUnload(_In_ FLT_FILTER_UNLOAD_FLAGS Flags)
{
    UNREFERENCED_PARAMETER(Flags);

    if (g_FuckScanData.ServerPort != NULL) {
        FltCloseCommunicationPort(g_FuckScanData.ServerPort);
    }

    if (g_FuckScanData.FilterHandle != NULL) {
        FltUnregisterFilter(g_FuckScanData.FilterHandle);
    }

    ExDeleteResourceLite(&g_Context.RulesLock);

    return STATUS_SUCCESS;
}

//
// Driver Entry
//

NTSTATUS DriverEntry(_In_ PDRIVER_OBJECT DriverObject, _In_ PUNICODE_STRING RegistryPath)
{
    NTSTATUS status;
    OBJECT_ATTRIBUTES oa;
    PSECURITY_DESCRIPTOR sd;
    UNICODE_STRING portName;

    UNREFERENCED_PARAMETER(RegistryPath);

    RtlZeroMemory(&g_Context, sizeof(g_Context));
    ExInitializeResourceLite(&g_Context.RulesLock);

    status = FltRegisterFilter(DriverObject, &FilterRegistration, &g_FuckScanData.FilterHandle);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    status = FltBuildDefaultSecurityDescriptor(&sd, FLT_PORT_ALL_ACCESS);
    if (!NT_SUCCESS(status)) {
        goto Cleanup;
    }

    RtlInitUnicodeString(&portName, FUCKSCAN_PORT_NAME);

    InitializeObjectAttributes(&oa, &portName, OBJ_KERNEL_HANDLE | OBJ_CASE_INSENSITIVE, NULL, sd);

    status = FltCreateCommunicationPort(g_FuckScanData.FilterHandle, &g_FuckScanData.ServerPort, &oa, NULL, FuckScanPortConnect, FuckScanPortDisconnect, FuckScanPortMessage, 1);

    FltFreeSecurityDescriptor(sd);

    if (!NT_SUCCESS(status)) {
        goto Cleanup;
    }

    status = FltStartFiltering(g_FuckScanData.FilterHandle);
    if (!NT_SUCCESS(status)) {
        goto Cleanup;
    }

    return STATUS_SUCCESS;

Cleanup:
    if (g_FuckScanData.ServerPort != NULL) {
        FltCloseCommunicationPort(g_FuckScanData.ServerPort);
    }

    if (g_FuckScanData.FilterHandle != NULL) {
        FltUnregisterFilter(g_FuckScanData.FilterHandle);
    }

    ExDeleteResourceLite(&g_Context.RulesLock);

    return status;
}
