{ bash, fetchFromGitHub, lib, lld, makeWrapper, onnxruntime, openssl, perl, pkg-config, runCommand, rustPlatform, sqlite }:

let
  manifest = builtins.fromJSON (builtins.readFile ./package-manifest.json);
  upstreamSrc = fetchFromGitHub {
    owner = manifest.source.owner;
    repo = manifest.source.repo;
    rev = manifest.source.rev;
    hash = manifest.source.hash;
  };
  asupersyncSrc = fetchFromGitHub {
    owner = manifest.source.siblings.asupersync.owner;
    repo = manifest.source.siblings.asupersync.repo;
    rev = manifest.source.siblings.asupersync.rev;
    hash = manifest.source.siblings.asupersync.hash;
  };
  beadsRustSrc = fetchFromGitHub {
    owner = manifest.source.siblings.beads_rust.owner;
    repo = manifest.source.siblings.beads_rust.repo;
    rev = manifest.source.siblings.beads_rust.rev;
    hash = manifest.source.siblings.beads_rust.hash;
  };
  frankensqliteSrc = fetchFromGitHub {
    owner = manifest.source.siblings.frankensqlite.owner;
    repo = manifest.source.siblings.frankensqlite.repo;
    rev = manifest.source.siblings.frankensqlite.rev;
    hash = manifest.source.siblings.frankensqlite.hash;
  };
  frankentuilSrc = fetchFromGitHub {
    owner = manifest.source.siblings.frankentui.owner;
    repo = manifest.source.siblings.frankentui.repo;
    rev = manifest.source.siblings.frankentui.rev;
    hash = manifest.source.siblings.frankentui.hash;
  };
  sqlmodelRustSrc = fetchFromGitHub {
    owner = manifest.source.siblings.sqlmodel_rust.owner;
    repo = manifest.source.siblings.sqlmodel_rust.repo;
    rev = manifest.source.siblings.sqlmodel_rust.rev;
    hash = manifest.source.siblings.sqlmodel_rust.hash;
  };
  sourceRoot = runCommand "${manifest.binary.name}-${manifest.source.version}-src" { } ''
    mkdir -p "$out/upstream" "$out/asupersync" "$out/beads_rust" \
             "$out/frankensqlite" "$out/frankentui" "$out/sqlmodel_rust"
    cp -R ${upstreamSrc}/. "$out/upstream/"
    cp -R ${asupersyncSrc}/. "$out/asupersync/"
    cp -R ${beadsRustSrc}/. "$out/beads_rust/"
    cp -R ${frankensqliteSrc}/. "$out/frankensqlite/"
    cp -R ${frankentuilSrc}/. "$out/frankentui/"
    cp -R ${sqlmodelRustSrc}/. "$out/sqlmodel_rust/"
  '';
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
  src = sourceRoot;
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
