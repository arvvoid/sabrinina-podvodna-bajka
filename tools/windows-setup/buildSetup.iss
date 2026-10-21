; Compile with Inno Setup 6. Paths are relative to this script.
#define AppName "Sabrina's Underwater Fairy Tale"
#ifndef AppVersion
  #define AppVersion GetVersionNumbersString(SourcePath + "setup-source\sabrina.exe")
#endif

[Setup]
AppId={{B6241701-11B8-4E51-99FC-470EDB10BDBA}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=arvvoid
AppPublisherURL=https://github.com/arvvoid/sabrinina-podvodna-bajka
DefaultDirName={localappdata}\Programs\Sabrina
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=.
OutputBaseFilename=Sabrina-{#AppVersion}-Setup
SignTool=LDTSign
SignedUninstaller=yes
SetupIconFile=..\..\art\icons\app_icon.ico
UninstallDisplayIcon={app}\app_icon.ico
WizardStyle=modern
LicenseFile=setup-source\LICENSE
InfoBeforeFile=setup-source\NOTICE
Compression=lzma2
SolidCompression=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "setup-source\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\art\icons\app_icon.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "setup-source\licenses\ASSET_LICENSE.txt"; Flags: dontcopy
Source: "setup-source\licenses\LICENSE-DejaVu.txt"; Flags: dontcopy
Source: "setup-source\licenses\LICENSE-FluidR3.txt"; Flags: dontcopy

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\sabrina.exe"; WorkingDir: "{app}"; IconFilename: "{app}\app_icon.ico"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\sabrina.exe"; WorkingDir: "{app}"; IconFilename: "{app}\app_icon.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\sabrina.exe"; Description: "Launch {#AppName}"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent unchecked

[Code]
var
  LicensePages: array[0..2] of TOutputMsgMemoWizardPage;
  LicenseAccepted: array[0..2] of TNewCheckBox;

function InitializeSetup: Boolean;
begin
  Result := not WizardSilent;
  if not Result then
    SuppressibleMsgBox(
      'Please run Setup interactively to review and accept all four licences.',
      mbError, MB_OK, IDOK);
end;

procedure UpdateLicenseAcceptance(Sender: TObject);
var
  I: Integer;
begin
  for I := 0 to 2 do
    if WizardForm.CurPageID = LicensePages[I].ID then
      WizardForm.NextButton.Enabled := LicenseAccepted[I].Checked;
end;

procedure AddLicensePage(Index, AfterID: Integer; FileName, Title: String);
var
  Text: AnsiString;
begin
  ExtractTemporaryFile(FileName);
  if not LoadStringFromFile(ExpandConstant('{tmp}\') + FileName, Text) then
    RaiseException('Unable to read required licence: ' + FileName);
  LicensePages[Index] := CreateOutputMsgMemoPage(
    AfterID, Title, 'You must accept this licence before installing.',
    'Please review the complete text below.', UTF8Decode(Text));
  LicensePages[Index].RichEditViewer.Height :=
    LicensePages[Index].RichEditViewer.Height - ScaleY(40);
  LicenseAccepted[Index] := TNewCheckBox.Create(WizardForm);
  with LicenseAccepted[Index] do
  begin
    Parent := LicensePages[Index].Surface;
    Left := 0;
    Top := LicensePages[Index].SurfaceHeight - ScaleY(30);
    Width := LicensePages[Index].SurfaceWidth;
    Height := ScaleY(25);
    Caption := 'I accept the terms of this licence';
    OnClick := @UpdateLicenseAcceptance;
  end;
end;

procedure InitializeWizard;
begin
  AddLicensePage(0, wpLicense, 'ASSET_LICENSE.txt', 'Game content and asset licence');
  AddLicensePage(1, LicensePages[0].ID, 'LICENSE-DejaVu.txt', 'DejaVu font licence');
  AddLicensePage(2, LicensePages[1].ID, 'LICENSE-FluidR3.txt', 'FluidR3 soundfont licence');
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  UpdateLicenseAcceptance(nil);
end;

function NextButtonClick(CurPageID: Integer): Boolean;
var
  I: Integer;
begin
  Result := True;
  for I := 0 to 2 do
    if CurPageID = LicensePages[I].ID then
      Result := LicenseAccepted[I].Checked;
end;