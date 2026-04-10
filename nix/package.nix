{ bash, lib, lld, makeWrapper, onnxruntime, openssl, perl, pkg-config, rustPlatform, sqlite }:

let
  manifest = builtins.fromJSON (builtins.readFile ./package-manifest.json);
  sourceTree = lib.cleanSourceWith {
    src = ../.;
    filter = path: type:
      let
        base = baseNameOf path;
        excluded = [
          ".beads"
          ".claude"
          ".git"
          ".github"
          ".ntm"
          "~"
          "agent_baseline"
          "artifacts"
          "baselines"
          "docs"
          "e2e"
          "examples"
          "formal"
          "fuzz"
          "legacy_pydantic"
          "legacy_sqlalchemy"
          "legacy_sqlmodel"
          "legacy_sqlite_code"
          "packages"
          "result"
          "sample_beads_db_files"
          "sample_sqlite_db_files"
          "scripts"
          "skills"
          "temp_test"
          "temp_test_2"
          "tests"
          "tools"
          "vendor"
        ];
      in
      !(builtins.elem base excluded);
  };
  builtBinary = manifest.binary.upstreamName or manifest.binary.name;
  aliasOutputs = manifest.binary.aliases or [ ];
  licenseMap = {
    "MIT" = lib.licenses.mit;
    "Apache-2.0" = lib.licenses.asl20;
  };
  resolvedLicense =
    if builtins.hasAttr manifest.meta.licenseSpdx licenseMap
    then licenseMap.${manifest.meta.licenseSpdx}
    else lib.licenses.unfree;
  aliasScripts = lib.concatMapStrings
    (
      alias:
      ''
        cat > "$out/bin/${alias}" <<EOF
#!${lib.getExe bash}
exec "$out/bin/${manifest.binary.name}" "\$@"
EOF
        chmod +x "$out/bin/${alias}"
      ''
    )
    aliasOutputs;
in
rustPlatform.buildRustPackage {
  pname = manifest.binary.name;
  version = manifest.package.version;
  src = sourceTree;
  sourceRoot = "source/upstream";

  cargoLock = {
    lockFile = ../upstream/Cargo.lock;
    allowBuiltinFetchGit = true;
  };

  cargoBuildFlags =
    (lib.optionals (manifest.binary ? package) [ "-p" manifest.binary.package ])
    ++ [ "--bin=${builtBinary}" ];

  nativeBuildInputs = [ lld makeWrapper perl pkg-config ];
  buildInputs = [ onnxruntime openssl sqlite ];
  doCheck = false;

  env = {
    ORT_LIB_LOCATION = "${lib.getLib onnxruntime}/lib";
    ORT_PREFER_DYNAMIC_LINK = "1";
    ORT_STRATEGY = "system";
    RUSTC_BOOTSTRAP = "1";
    VERGEN_IDEMPOTENT = "1";
    VERGEN_GIT_SHA = manifest.source.rev;
    VERGEN_GIT_DIRTY = "false";
  };

  postInstall = ''
    if [ "${builtBinary}" != "${manifest.binary.name}" ]; then
      mv "$out/bin/${builtBinary}" "$out/bin/${manifest.binary.name}"
    fi
    wrapProgram "$out/bin/${manifest.binary.name}" \
      --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath [ onnxruntime ]}" \
      --set ORT_LIB_LOCATION "${lib.getLib onnxruntime}/lib" \
      --set ORT_PREFER_DYNAMIC_LINK "1" \
      --set ORT_STRATEGY "system"
    ${aliasScripts}
  '';

  meta = with lib; {
    description = manifest.meta.description;
    homepage = manifest.meta.homepage;
    license = resolvedLicense;
    mainProgram = manifest.binary.name;
    platforms = platforms.linux ++ platforms.darwin;
  };
}
