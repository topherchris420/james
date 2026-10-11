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

[Setup]
AppId={{CC1BEDD1-3479-4B46-A3C3-A47DC4398EE2}
AppName=James
AppVersion={#AppVersion}
AppPublisher=James contributors
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
UninstallDisplayIcon={app}\james.exe
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

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Icons]
Name: "{group}\James"; Filename: "{app}\james.exe"
Name: "{autodesktop}\James"; Filename: "{app}\james.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\james.exe"; Description: "Open James"; Flags: nowait postinstall skipifsilent
