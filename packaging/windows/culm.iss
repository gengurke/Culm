; Inno Setup script for Culm-<version>-windows-x64-setup.exe (F8.5). Installs for the current user only, without
; admin rights, into %LOCALAPPDATA%\Programs\Culm, with a Start menu entry, an optional desktop icon and an uninstaller.
;
;   iscc /DVersion=<version> /DExe=<path to culm.exe> /O<output dir> packaging\windows\culm.iss
;
; The uninstaller removes the program only: settings, installed-addon records and backups stay in AppData.

#ifndef Version
  #error Pass /DVersion=<version>
#endif
#ifndef Exe
  #error Pass /DExe=<path to culm.exe>
#endif

[Setup]
; Never change the AppId: Windows recognises installed versions by it, so updates install over the old one.
AppId={{93499F2C-AB19-4438-9347-76959BE96186}
AppName=Culm
AppVersion={#Version}
AppVerName=Culm {#Version}
AppPublisher=Culm
AppPublisherURL=https://github.com/gengurke/Culm
AppSupportURL=https://github.com/gengurke/Culm/issues
AppUpdatesURL=https://github.com/gengurke/Culm/releases
LicenseFile=..\..\LICENSE
PrivilegesRequired=lowest
DefaultDirName={autopf}\Culm
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
SetupIconFile=..\..\crates\app\assets\icon\culm.ico
UninstallDisplayIcon={app}\culm.exe
UninstallDisplayName=Culm
WizardStyle=modern
OutputBaseFilename=Culm-{#Version}-windows-x64-setup

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#Exe}"; DestDir: "{app}"; DestName: "culm.exe"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"

[Icons]
Name: "{autoprograms}\Culm"; Filename: "{app}\culm.exe"
Name: "{autodesktop}\Culm"; Filename: "{app}\culm.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\culm.exe"; Description: "{cm:LaunchProgram,Culm}"; Flags: nowait postinstall skipifsilent
