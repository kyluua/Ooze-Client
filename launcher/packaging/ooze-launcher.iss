[Setup]
AppId=OozeClient.OozeLauncher
AppName=Ooze Launcher
AppVersion=0.1.0-rc1
AppPublisher=Ooze Client
DefaultDirName={localappdata}\Programs\Ooze Launcher
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
WizardStyle=modern
OutputBaseFilename=Ooze-Launcher-0.1.0-rc1-Windows-x64-Setup
UninstallDisplayIcon={app}\ooze-launcher.exe
CloseApplications=no
Compression=lzma2
SolidCompression=yes

[Files]
Source: "{#PayloadDirectory}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{autoprograms}\Ooze Launcher"; Filename: "{app}\ooze-launcher.exe"; WorkingDir: "{app}"
