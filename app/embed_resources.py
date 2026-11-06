import struct
import ctypes
import ctypes.wintypes
import sys
import os

kernel32 = ctypes.windll.kernel32
kernel32.BeginUpdateResourceW.restype = ctypes.wintypes.HANDLE
kernel32.BeginUpdateResourceW.argtypes = [ctypes.c_wchar_p, ctypes.wintypes.BOOL]
kernel32.UpdateResourceW.restype = ctypes.wintypes.BOOL
kernel32.UpdateResourceW.argtypes = [ctypes.wintypes.HANDLE, ctypes.c_void_p, ctypes.c_void_p, ctypes.wintypes.WORD, ctypes.c_char_p, ctypes.wintypes.DWORD]
kernel32.EndUpdateResourceW.restype = ctypes.wintypes.BOOL
kernel32.EndUpdateResourceW.argtypes = [ctypes.wintypes.HANDLE, ctypes.wintypes.BOOL]

def embed_ico(exe_path, ico_path):
    with open(ico_path, 'rb') as f:
        ico_data = f.read()

    id_reserved, id_type, id_count = struct.unpack('<HHH', ico_data[:6])
    assert id_type == 1, 'Not an icon file'

    h_update = kernel32.BeginUpdateResourceW(exe_path, False)
    if not h_update:
        raise ctypes.WinError()

    grp_entries = []
    offset = 6
    for i in range(id_count):
        width, height, color_count, reserved, planes, bit_count, bytes_in_res, image_offset = struct.unpack(
            '<BBBBHHII', ico_data[offset:offset+16]
        )
        offset += 16
        icon_id = i + 1
        grp_entries.append(struct.pack('<BBBBHHIH', width, height, color_count, reserved, planes, bit_count, bytes_in_res, icon_id))

        img_data = ico_data[image_offset:image_offset+bytes_in_res]
        res = kernel32.UpdateResourceW(
            h_update,
            ctypes.c_void_p(3),
            ctypes.c_void_p(icon_id),
            1033,
            img_data,
            len(img_data)
        )
        if not res:
            raise ctypes.WinError()

    grp_header = struct.pack('<HHH', id_reserved, id_type, id_count) + b''.join(grp_entries)
    res = kernel32.UpdateResourceW(
        h_update,
        ctypes.c_void_p(14),
        ctypes.c_void_p(1),
        1033,
        grp_header,
        len(grp_header)
    )
    if not res:
        raise ctypes.WinError()

    if not kernel32.EndUpdateResourceW(h_update, False):
        raise ctypes.WinError()
    print('Embedded icon group into:', exe_path)

def embed_manifest(exe_path, manifest_path):
    import subprocess
    mt_exe = r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\mt.exe"
    if os.path.exists(mt_exe):
        subprocess.check_call([
            mt_exe, "-nologo",
            "-manifest", manifest_path,
            f"-outputresource:{exe_path};#1"
        ])
        print('Embedded manifest into:', exe_path)

if __name__ == '__main__':
    base_dir = os.path.dirname(os.path.abspath(__file__))
    exe = os.path.join(base_dir, 'target', 'release', 'everything-app.exe')
    ico = os.path.join(base_dir, 'assets', 'app.ico')
    manifest = os.path.join(base_dir, 'app.manifest')
    if os.path.exists(exe):
        embed_manifest(exe, manifest)
        embed_ico(exe, ico)
