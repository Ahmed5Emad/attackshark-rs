"""win_hid_probe.py — read-only HID probe for the V6 (pure stdlib ctypes).

Why this exists: the vendor hidapi.dll is 32-bit but every Python here is
64-bit, so hidapi.dll cannot be loaded. This talks to the SYSTEM hid.dll /
setupapi instead. Lesson learned: set restype/argtypes on
SetupDiGetClassDevsW (HANDLE is 64-bit; the ctypes default c_int truncates
it and enumeration silently returns zero devices).

Usage:  python tools/win_hid_probe.py
Read-only: opens devices with desired_access=0, never writes.
"""
import ctypes
from ctypes import wintypes

hid = ctypes.WinDLL("hid")
setup = ctypes.WinDLL("setupapi")
k32 = ctypes.WinDLL("kernel32")

setup.SetupDiGetClassDevsW.restype = ctypes.c_void_p
setup.SetupDiGetClassDevsW.argtypes = [
    ctypes.c_void_p, wintypes.LPCWSTR, wintypes.HWND, wintypes.DWORD]
setup.SetupDiEnumDeviceInterfaces.restype = wintypes.BOOL
setup.SetupDiEnumDeviceInterfaces.argtypes = [
    ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p,
    wintypes.DWORD, ctypes.c_void_p]
setup.SetupDiGetDeviceInterfaceDetailW.restype = wintypes.BOOL
hid.HidD_GetAttributes.restype = wintypes.BOOL


class GUID(ctypes.Structure):
    _fields_ = [("Data1", wintypes.DWORD), ("Data2", wintypes.WORD),
                ("Data3", wintypes.WORD), ("Data4", wintypes.BYTE * 8)]


class SP_DEVINFO_DATA(ctypes.Structure):
    _fields_ = [("cbSize", wintypes.DWORD), ("ClassGuid", GUID),
                ("DevInst", wintypes.DWORD), ("Reserved", ctypes.c_void_p)]


class SP_DEVICE_INTERFACE_DATA(ctypes.Structure):
    _fields_ = [("cbSize", wintypes.DWORD), ("InterfaceClassGuid", GUID),
                ("Flags", wintypes.DWORD), ("Reserved", ctypes.c_void_p)]


class HIDD_ATTRIBUTES(ctypes.Structure):
    _fields_ = [("Size", wintypes.ULONG), ("VendorID", wintypes.USHORT),
                ("ProductID", wintypes.USHORT),
                ("VersionNumber", wintypes.USHORT)]


GUID_DEVINTERFACE_HID = GUID(0x4D1E55B2, 0xF16F, 0x11CF,
                             (wintypes.BYTE * 8)(0x88, 0xCB, 0x00, 0x11,
                                                 0x11, 0x00, 0x00, 0x30))
INVALID = ctypes.c_void_p(-1).value


def main():
    hdev = ctypes.c_void_p(setup.SetupDiGetClassDevsW(
        ctypes.byref(GUID_DEVINTERFACE_HID), None, None, 0x2 | 0x10))
    if not hdev or hdev.value in (None, 0, INVALID):
        print("SetupDiGetClassDevsW failed, err=", k32.GetLastError())
        return
    found, i = [], 0
    while i < 64:
        iid = SP_DEVICE_INTERFACE_DATA()
        iid.cbSize = ctypes.sizeof(iid)
        if not setup.SetupDiEnumDeviceInterfaces(
                hdev, None, ctypes.byref(GUID_DEVINTERFACE_HID),
                i, ctypes.byref(iid)):
            err = k32.GetLastError()
            if err != 259:  # ERROR_NO_MORE_ITEMS
                print("enum stopped, err=", err)
            break
        need = wintypes.DWORD(0)
        setup.SetupDiGetDeviceInterfaceDetailW(
            hdev, ctypes.byref(iid), None, 0, ctypes.byref(need), None)
        buf = ctypes.create_string_buffer(need.value)
        ctypes.cast(buf, ctypes.POINTER(wintypes.DWORD))[0] = 8
        did = SP_DEVINFO_DATA()
        did.cbSize = ctypes.sizeof(did)
        if setup.SetupDiGetDeviceInterfaceDetailW(
                hdev, ctypes.byref(iid), buf, need, None,
                ctypes.byref(did)):
            path = ctypes.wstring_at(ctypes.addressof(buf) + 4)
            h = k32.CreateFileW(path, 0, 7, None, 3, 0x40000000, None)
            if h and h != INVALID and int(h) != -1:
                a = HIDD_ATTRIBUTES()
                a.Size = ctypes.sizeof(a)
                if hid.HidD_GetAttributes(h, ctypes.byref(a)):
                    tag = ("  <-- V6-WIRED" if (a.VendorID == 0x1D57
                                                and a.ProductID == 0x2224)
                           else ("  <-- V6-2.4G" if a.VendorID == 0x1D57
                                 and a.ProductID in (0xFA60, 0xFA65) else ""))
                    print(f"VID_{a.VendorID:04x} PID_{a.ProductID:04x}{tag}")
                    print(f"  {path}")
                    found.append((a.VendorID, a.ProductID))
                k32.CloseHandle(h)
        i += 1
    setup.SetupDiDestroyDeviceInfoList(hdev)
    print(f"TOTAL hid={len(found)}")


if __name__ == "__main__":
    main()
