; ZeroLaunch Inno Setup 安装包。
; xtask 通过 /DAppVersion、/DTargetTriple、/DArchLabel、/DRepoRoot 注入构建参数。

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef TargetTriple
  #define TargetTriple "x86_64-pc-windows-msvc"
#endif
#ifndef ArchLabel
  #define ArchLabel "x64"
#endif
#ifndef RepoRoot
  #define RepoRoot ".."
#endif

[Setup]
AppId={{8D3F4A80-5B9C-4F2E-9A61-2C7B5E4D1A03}
AppName=ZeroLaunch
AppVersion={#AppVersion}
AppPublisher=ghost-him
AppPublisherURL=https://github.com/ghost-him/ZeroLaunch-rs
AppSupportURL=https://github.com/ghost-him/ZeroLaunch-rs
AppUpdatesURL=https://github.com/ghost-him/ZeroLaunch-rs/releases
DefaultDirName={autopf}\ZeroLaunch
DefaultGroupName=ZeroLaunch
DisableProgramGroupPage=yes
LicenseFile={#RepoRoot}\LICENSE.txt
SetupIconFile={#RepoRoot}\app\icons\icon.ico
OutputDir={#RepoRoot}
OutputBaseFilename=ZeroLaunch-{#AppVersion}-{#ArchLabel}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
#if ArchLabel == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#RepoRoot}\target\{#TargetTriple}\release\zerolaunch-gpui.exe"; DestDir: "{app}"; DestName: "ZeroLaunch.exe"; Flags: ignoreversion
Source: "{#RepoRoot}\app\icons\*"; DestDir: "{app}\icons"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RepoRoot}\crates\zerolaunch-rs\locales\*"; DestDir: "{app}\locales"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RepoRoot}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}\NOTICE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}\PRIVACY.md"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
; 升级时移除旧宿主文件名。
Type: files; Name: "{app}\zerolaunch-rs.exe"
Type: files; Name: "{app}\zerolaunch-gpui.exe"

[Icons]
Name: "{group}\ZeroLaunch"; Filename: "{app}\ZeroLaunch.exe"
Name: "{autodesktop}\ZeroLaunch"; Filename: "{app}\ZeroLaunch.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\zerolaunch-rs"; ValueType: string; ValueName: ""; ValueData: "URL:ZeroLaunch Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\zerolaunch-rs"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\zerolaunch-rs\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\ZeroLaunch.exe,0"
Root: HKCU; Subkey: "Software\Classes\zerolaunch-rs\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\ZeroLaunch.exe"" ""%1"""

[Run]
Filename: "{app}\ZeroLaunch.exe"; Description: "{cm:LaunchProgram,ZeroLaunch}"; Flags: nowait postinstall skipifsilent

[Code]
const
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall';

function IsOldProduct(const DisplayName: String): Boolean;
begin
  Result := Pos('zerolaunch-rs', Lowercase(DisplayName)) > 0;
end;

procedure StopOldHost;
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM zerolaunch-rs.exe /T', '', SW_HIDE,
    ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM zerolaunch-gpui.exe /T', '', SW_HIDE,
    ewWaitUntilTerminated, ResultCode);
end;

procedure UninstallOldProducts;
var
  Roots: array[0..2] of Integer;
  RootIndex: Integer;
  Names: TArrayOfString;
  I: Integer;
  Key: String;
  DisplayName: String;
  ProductCode: String;
  ResultCode: Integer;
begin
  Roots[0] := HKLM;
  Roots[1] := HKLM32;
  Roots[2] := HKCU;
  for RootIndex := 0 to 2 do
  begin
    if RegGetSubkeyNames(Roots[RootIndex], UninstallKey, Names) then
    begin
      for I := 0 to GetArrayLength(Names) - 1 do
      begin
        Key := UninstallKey + '\' + Names[I];
        if RegQueryStringValue(Roots[RootIndex], Key, 'DisplayName', DisplayName) then
        begin
          if IsOldProduct(DisplayName) and (Copy(Names[I], 1, 1) = '{') then
          begin
            ProductCode := Names[I];
            Log('Uninstalling old ZeroLaunch-rs MSI: ' + ProductCode);
            Exec('msiexec.exe', '/x ' + ProductCode + ' /qn /norestart', '',
              SW_HIDE, ewWaitUntilTerminated, ResultCode);
          end;
        end;
      end;
    end;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopOldHost;
  UninstallOldProducts;
  Result := '';
end;
