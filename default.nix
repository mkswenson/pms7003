{ lib
, naersk
, stdenv
, clangStdenv
, hostPlatform
, targetPlatform
, pkg-config
, libiconv
, rustfmt
, cargo
, rustc
, udev
}:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
in

naersk.lib."${targetPlatform.system}".buildPackage rec {
  src = ./.;

  nativeBuildInputs = [ pkg-config ];

  buildInputs = [
    rustfmt
    pkg-config
    cargo
    rustc
    libiconv
    udev
  ];
  checkInputs = [ cargo rustc ];

  doCheck = true;
  CARGO_BUILD_INCREMENTAL = "false";
  RUST_BACKTRACE = "full";
  copyLibs = true;

  # Optional things you might need:
  #
  # If you depend on `libclang`:
  # LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  #
  # If you depend on protobuf:
  # PROTOC = "${protobuf}/bin/protoc";
  # PROTOC_INCLUDE = "${protobuf}/include";
  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;

  name = cargoToml.package.name;
  version = cargoToml.package.version;

  meta = with lib; {
    description = cargoToml.package.description;
    homepage = cargoToml.package.homepage;
    license = with licenses; [ agpl3Plus ];
    maintainers = with maintainers; [ ];
  };
}
