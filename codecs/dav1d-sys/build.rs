use std::{env, fs, path::{Path, PathBuf}, process::Command};

fn build(root: &Path, out: &Path) -> cc::Build {
    let mut b = cc::Build::new();
    b.include(root).include(root.join("include")).include(out).warnings(false).flag_if_supported("-std=c11");
    b
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../vendor/dav1d");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    assert!(arch == "aarch64" || arch == "x86_64");
    let arm = arch == "aarch64";
    let linux = os == "linux";
    let mut config = String::new();
    for (key, value) in [
        ("CONFIG_8BPC", true), ("CONFIG_16BPC", true), ("HAVE_ASM", true), ("CONFIG_ASM_DEBUG", false),
        ("TRIM_DSP_FUNCTIONS", true), ("CONFIG_LOG", true), ("HAVE_CLOCK_GETTIME", true),
        ("HAVE_SIGACTION", true), ("HAVE_POSIX_MEMALIGN", true), ("HAVE_MEMALIGN", linux),
        ("HAVE_ALIGNED_ALLOC", true), ("HAVE_DLSYM", linux), ("HAVE_SYS_TYPES_H", true),
        ("HAVE_UNISTD_H", true), ("HAVE_IO_H", false), ("HAVE_PTHREAD_NP_H", false),
        ("HAVE_GETAUXVAL", linux), ("HAVE_ELF_AUX_INFO", false), ("HAVE_PTHREAD_GETAFFINITY_NP", linux),
        ("HAVE_PTHREAD_SETAFFINITY_NP", linux), ("HAVE_PTHREAD_SETNAME_NP", true), ("HAVE_PTHREAD_SET_NAME_NP", false),
        ("HAVE_C11_GENERIC", true), ("ENDIANNESS_BIG", false), ("ARCH_AARCH64", arm), ("ARCH_ARM", false),
        ("ARCH_X86", !arm), ("ARCH_X86_64", !arm), ("ARCH_X86_32", false), ("ARCH_PPC64LE", false),
        ("ARCH_RISCV", false), ("ARCH_RV32", false), ("ARCH_RV64", false), ("ARCH_LOONGARCH", false),
        ("ARCH_LOONGARCH32", false), ("ARCH_LOONGARCH64", false), ("HAVE_AS_FUNC", arm && linux),
        ("HAVE_AS_ARCH_DIRECTIVE", arm), ("HAVE_AS_ARCHEXT_DOTPROD_DIRECTIVE", arm),
        ("HAVE_AS_ARCHEXT_I8MM_DIRECTIVE", arm), ("HAVE_AS_ARCHEXT_SVE_DIRECTIVE", arm),
        ("HAVE_AS_ARCHEXT_SVE2_DIRECTIVE", arm), ("HAVE_DOTPROD", arm), ("HAVE_I8MM", arm), ("HAVE_SVE", arm), ("HAVE_SVE2", arm),
    ] { config.push_str(&format!("#define {key} {}\n", u8::from(value))); }
    if !linux { config.push_str("#define PREFIX 1\n"); }
    if arm { config.push_str("#define PIC 3\n#define AS_ARCH_LEVEL armv8.6-a+crc\n"); }
    fs::write(out.join("config.h"), config).unwrap();
    fs::write(out.join("vcs_version.h"), "#define DAV1D_VERSION \"1.5.4\"\n").unwrap();
    let common = "cdf cpu ctx data decode dequant_tables getbits intra_edge itx_1d lf_mask lib log mem msac obu pal picture qm ref refmvs scan tables thread_task warpmv wedge";
    let templates = "cdef_apply cdef fg_apply filmgrain ipred_prepare ipred itx lf_apply loopfilter looprestoration lr_apply mc recon";
    for depth in [8, 16] {
        let mut b = build(&root, &out);
        b.define("BITDEPTH", Some(if depth == 8 { "8" } else { "16" }));
        for name in templates.split_whitespace() { b.file(root.join(format!("src/{name}_tmpl.c"))); }
        b.compile(&format!("dav1d_{depth}"));
    }
    let mut b = build(&root, &out);
    for name in common.split_whitespace() { b.file(root.join(format!("src/{name}.c"))); }
    if arm {
        b.file(root.join("src/arm/cpu.c"));
        let mut assembly = build(&root, &out);
        for name in "itx looprestoration_common msac refmvs cdef filmgrain ipred loopfilter looprestoration mc mc_dotprod cdef16 filmgrain16 ipred16 itx16 loopfilter16 looprestoration16 mc16 mc16_sve".split_whitespace() {
            assembly.file(root.join(format!("src/arm/64/{name}.S")));
        }
        assembly.compile("dav1d_arm");
    } else {
        b.file(root.join("src/x86/cpu.c"));
        fs::write(out.join("config.asm"), format!("%define private_prefix dav1d\n%define FORCE_VEX_ENCODING 0\n%define ARCH_X86_64 1\n%define ARCH_X86_32 0\n%define PIC 1\n%define PREFIX {}\n", u8::from(os == "macos"))).unwrap();
        let names = "cpuid msac pal refmvs itx_avx512 cdef_avx2 cdef_avx512 itx_avx2 cdef_sse itx_sse filmgrain_avx512 ipred_avx512 loopfilter_avx512 looprestoration_avx512 mc_avx512 filmgrain_avx2 ipred_avx2 loopfilter_avx2 looprestoration_avx2 mc_avx2 filmgrain_sse ipred_sse loopfilter_sse looprestoration_sse mc_sse cdef16_avx512 filmgrain16_avx512 ipred16_avx512 itx16_avx512 loopfilter16_avx512 looprestoration16_avx512 mc16_avx512 cdef16_avx2 filmgrain16_avx2 ipred16_avx2 loopfilter16_avx2 looprestoration16_avx2 mc16_avx2 cdef16_sse filmgrain16_sse ipred16_sse loopfilter16_sse looprestoration16_sse mc16_sse";
        for name in names.split_whitespace() {
            let obj = out.join(format!("{name}.o"));
            assert!(Command::new(env::var_os("NASM").unwrap_or_else(|| "nasm".into())).arg("-f").arg(if os == "macos" { "macho64" } else { "elf64" }).arg("-I").arg(format!("{}/src/", root.display())).arg("-I").arg(format!("{}/", out.display())).arg(root.join(format!("src/x86/{name}.asm"))).arg("-o").arg(&obj).status().unwrap().success());
            b.object(obj);
        }
    }
    b.file("shim.c");
    b.compile("dav1d");
    println!("cargo:rerun-if-changed={}", root.display());
    println!("cargo:rerun-if-changed=shim.c");
    println!("cargo:rerun-if-env-changed=NASM");
}
