; Inno Setup script for the HBC-2 IDE. Build it with installer.cmd (at the
; root of the repository), which passes the version and builds the programs
; first.
;
; Installed: the IDE, the command-line tools (hbc2.exe) and the C compiler
; (compiler\: its tools, startup code, runtime, library and peephole rules),
; which the IDE finds next to its executable.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define Root ".."
#define Release Root + "\hbc2\target\release"
#define Compiler Root + "\compiler"

[Setup]
; the AppId identifies the program for upgrades and uninstallation: never change it
AppId={{6F2B8C1E-4D7A-4E3B-9A51-2C8E7F0D3B64}
AppName=HBC-2 IDE
AppVersion={#AppVersion}
AppVerName=HBC-2 IDE {#AppVersion}
AppPublisher=Gianni LECLERCQ
DefaultDirName={autopf}\HBC-2 IDE
DefaultGroupName=HBC-2 IDE
DisableProgramGroupPage=yes
; installs in Program Files for every user: needs administrator rights
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=output
OutputBaseFilename=HBC-2-IDE-{#AppVersion}-setup
SetupIconFile={#Root}\hbc2\crates\hbc2-ide\assets\icon.ico
UninstallDisplayIcon={app}\hbc2-ide.exe
LicenseFile={#Root}\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; closes a running IDE before replacing its files
CloseApplications=yes

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#Release}\hbc2-ide.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Release}\hbc2.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Compiler}\bin\*.exe"; DestDir: "{app}\compiler\bin"; Flags: ignoreversion
Source: "{#Compiler}\runtime\*.has"; DestDir: "{app}\compiler\runtime"; Flags: ignoreversion
Source: "{#Compiler}\fcc\rules.hbc2"; DestDir: "{app}\compiler\fcc"; Flags: ignoreversion
Source: "{#Compiler}\lib\lib.has"; DestDir: "{app}\compiler\lib"; Flags: ignoreversion
Source: "{#Compiler}\lib\include\*.h"; DestDir: "{app}\compiler\lib\include"; Flags: ignoreversion
Source: "{#Compiler}\lib\src\*.c"; DestDir: "{app}\compiler\lib\src"; Flags: ignoreversion
Source: "{#Compiler}\lib\asm\*.has"; DestDir: "{app}\compiler\lib\asm"; Flags: ignoreversion recursesubdirs
Source: "{#Root}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
; files of an older version that this one no longer has
Type: filesandordirs; Name: "{app}\compiler"

[Icons]
Name: "{autoprograms}\HBC-2 IDE"; Filename: "{app}\hbc2-ide.exe"
Name: "{autodesktop}\HBC-2 IDE"; Filename: "{app}\hbc2-ide.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\hbc2-ide.exe"; Description: "{cm:LaunchProgram,HBC-2 IDE}"; Flags: nowait postinstall skipifsilent
