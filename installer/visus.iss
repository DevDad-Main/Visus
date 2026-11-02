#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{7B1E3E52-4C3A-4B63-9F0D-5A5B8C1D2E10}
AppName=Visus
AppVersion={#AppVersion}
AppPublisher=DevDad
AppPublisherURL=https://github.com/DevDad-Main/Visus
DefaultDirName={localappdata}\Programs\Visus
DefaultGroupName=Visus
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..
OutputBaseFilename=Visus-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\Visus.exe
SetupIconFile=..\crates\visus-app\assets\visus.ico
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
RestartApplications=no
#ifdef SIGN
SignTool=visus_sign
SignedUninstaller=yes
SignToolRetryCount=3
#endif

[Tasks]
Name: "autostart"; Description: "Start Visus when I sign in"; GroupDescription: "Options:"
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Options:"; Flags: unchecked

[Files]
Source: "..\Visus.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\PresentMon.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\PresentMon-LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Visus"; Filename: "{app}\Visus.exe"
Name: "{autodesktop}\Visus"; Filename: "{app}\Visus.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Visus"; ValueData: """{app}\Visus.exe"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\Visus.exe"; Description: "Launch Visus"; Flags: nowait postinstall; Check: ShouldLaunch

[Code]
procedure StopVisus();
var
  code: Integer;
begin
  Exec('taskkill', '/F /IM Visus.exe', '', SW_HIDE, ewWaitUntilTerminated, code);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopVisus();
  Result := '';
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  RunValue: String;
begin
  if CurStep = ssPostInstall then
    if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Visus', RunValue) then
      RegWriteStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Visus', '"' + ExpandConstant('{app}\Visus.exe') + '"');
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    StopVisus();
end;

function ShouldLaunch(): Boolean;
begin
  Result := (not WizardSilent()) or (ExpandConstant('{param:RESTART|0}') = '1');
end;
