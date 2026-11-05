# Everything Mechanics and Constants

Extracted and decompiled from `C:\Program Files\Everything 1.5a\Everything.exe` using Ghidra clean-room decompilation in Craft Anything.

## 1. NTFS & USN Change Journal Mechanics
- **FSCTL_QUERY_USN_JOURNAL Control Code**: `0x000900f4` (`589940`)
  - Function: `0x1401a9690` (Evidence `ev_25fd63509a7f538d2affc39f3f110432ea57c390eae352e765e086d7a407fdf0`)
  - Pseudocode: `FUN_1401b33b0(param_1, 0x900f4, 0, 0, local_48, 0x38, 0, 0)`
  - Used with `DeviceIoControl` to query USN journal metadata from volume handles.
- **FSCTL_READ_USN_JOURNAL Control Code**: `0x000900f8` (`589944`)
  - Function: `0x1401a9690` (Evidence `ev_25fd63509a7f538d2affc39f3f110432ea57c390eae352e765e086d7a407fdf0`)
  - Pseudocode: `FUN_1401b33b0(param_1, 0x900f8, &local_58, 0x10, 0, 0, 0, 0)`
- **FSCTL_ENUM_USN_DATA Control Code**: `0x000900b3` (`589875`)
  - Function: `0x140091c15` / `FUN_140091b90` (Evidence `ev_420b8d932e27d7dc6e5fb2371099a3da4136af8c89d18e6c3b60c05c5e86b8e5`)
  - Used to enumerate all Master File Table (MFT) records in rapid succession.
- **USN Buffer Size**: `65536` bytes (`0x10000`)
  - Function: `0x140091b90` (Evidence `ev_420b8d932e27d7dc6e5fb2371099a3da4136af8c89d18e6c3b60c05c5e86b8e5`)
  - Pseudocode: `puVar5 = (undefined8 *)FUN_1401959a0(0x10000)`
- **sizeof(USN_JOURNAL_DATA_V0)**: `56` bytes (`0x38`)
  - Function: `0x1401a9690` (Evidence `ev_25fd63509a7f538d2affc39f3f110432ea57c390eae352e765e086d7a407fdf0`)
  - Buffer length argument in `DeviceIoControl`: `0x38`

## 2. Window and UI Proportions
- **Default Window Width**: `1022`
  - Function: `0x1403b0080` (Evidence `ev_e6ba1b3824004585c708eb5e908d969e8b55777c726937dda09ed74e93bd38a2`) / `Session-1.5a.json`
- **Default Window Height**: `713`
  - Function: `0x1403b0080` (Evidence `ev_e6ba1b3824004585c708eb5e908d969e8b55777c726937dda09ed74e93bd38a2`) / `Session-1.5a.json`
- **Filters Dropdown Width**: `128`
  - Source: `Everything-1.5a.ini`
- **Sidebar Width**: `256`
  - Source: `Session-1.5a.json`
- **Thumbnail Size**: `128`
  - Source: `Session-1.5a.json`
- **Default Columns**: `Name` (width 350), `Path` (width 996), `Size` (width 168), `Date Modified` (width 163)
  - Source: `Session-1.5a.json`

## 3. Search and Engine Limits
- **Search Max Results**: `2000` (for streaming IPC) / unbounded in full GUI view
  - Source: `files-explorer/src-tauri/src/core/types.rs`
- **Search Batch Size**: `500`
  - Source: `files-explorer/src-tauri/src/core/types.rs`
- **Fast Sort Multi-threaded**: enabled
  - Source: `function 0x1403dbcb4` (`os_sort_MT`)
