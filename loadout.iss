; LOADOUT installer (Inno Setup). Same shape as FRAGO's frago.iss: per-user
; into LocalAppData, no admin, no UAC. AppName is the AppId, so Apps & features
; gets its own "LOADOUT_is1" key. Profiles live in {app}\profiles by default;
; the uninstaller removes only what it installed, so they survive it.

[Setup]
AppName=LOADOUT
AppVersion=0.3.0
AppPublisher=Boring Tech Stuff LLC
DefaultDirName={localappdata}\LOADOUT
DefaultGroupName=ED HUMINT
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
DisableDirPage=no
OutputDir=..\transfer
OutputBaseFilename=LOADOUT_Setup
SetupIconFile=icons\loadout.ico
Compression=lzma2/max
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayName=LOADOUT
UninstallDisplayIcon={app}\loadout.exe
CloseApplications=yes

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
; package.ps1 stages everything, the MSVC runtime included, into dist\LOADOUT.
Source: "dist\LOADOUT\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion; Excludes: "*.log"

[Icons]
Name: "{group}\LOADOUT"; Filename: "{app}\loadout.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\LOADOUT"; Filename: "{app}\loadout.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\loadout.exe"; Description: "Launch LOADOUT"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; The app's own log, written beside the exe.
Type: files; Name: "{app}\loadout.log"
