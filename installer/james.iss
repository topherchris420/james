; Compile through scripts/build-installer.ps1; the Cargo workspace owns the version.
#ifndef AppVersion
  #error AppVersion is required
#endif
#ifndef SourceRoot
  #error SourceRoot is required
#endif
#ifndef AppBinary
  #error AppBinary is required
#endif
#ifndef OutputPath
  #error OutputPath is required
#endif
#ifndef Notices
  #error Notices is required
#endif

[Setup]
AppId={{CC1BEDD1-3479-4B46-A3C3-A47DC4398EE2}
AppName=James
AppVersion={#AppVersion}
AppPublisher=James contributors
AppCopyright=MIT License, James contributors
UninstallDisplayName=James
AppPublisherURL=https://github.com/topherchris420/james
AppSupportURL=https://github.com/topherchris420/james/issues
DefaultDirName={localappdata}\Programs\James
DefaultGroupName=James
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableWelcomePage=yes
SetupIconFile={#SourceRoot}\installer\james.ico
UninstallDisplayIcon={app}\james.ico
OutputDir={#OutputPath}
OutputBaseFilename=James-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Files]
Source: "{#AppBinary}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceRoot}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Notices}"; DestDir: "{app}"; DestName: "THIRD-PARTY-NOTICES.txt"; Flags: ignoreversion
Source: "{#SourceRoot}\installer\james.ico"; DestDir: "{app}"; Flags: ignoreversion

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Icons]
Name: "{group}\James"; Filename: "{app}\james.exe"; IconFilename: "{app}\james.ico"
Name: "{autodesktop}\James"; Filename: "{app}\james.exe"; IconFilename: "{app}\james.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\james.exe"; Description: "Open James"; Flags: nowait postinstall skipifsilent

[Code]
// Uninstalling removes the app. A person's conversations and settings stay
// unless they choose otherwise here; a silent uninstall never deletes them.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  DataDir: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    DataDir := ExpandConstant('{localappdata}\Vers3Dynamics\James');
    if DirExists(DataDir) and not UninstallSilent() then
    begin
      if MsgBox('Also delete your James conversations and settings from this computer?' + #13#10#13#10 +
        'They are in ' + DataDir + '.' + #13#10 +
        'Choose No to keep them for the next time you install James.',
        mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
      begin
        if not DelTree(DataDir, True, True, True) then
          MsgBox('Some files could not be deleted. You can delete ' + DataDir + ' yourself.', mbInformation, MB_OK);
      end;
    end;
  end;
end;
