plugins {
    id("com.android.application")
}

android {
    namespace = "com.voidwavelab.kalokorg.tablet"
    compileSdk = 34
    ndkVersion = "26.2.11394342"

    defaultConfig {
        applicationId = "com.voidwavelab.kalokorg.tablet"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            // Target 64-bit ARM tablets and x86_64 Android Tablet emulators
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isJniDebuggable = true
        }
    }

    sourceSets {
        getByName("main") {
            manifest.srcFile("src/main/AndroidManifest.xml")
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

// Custom Gradle task to compile the Rust `kalokorg-tablet` cdylib (`libkalokorg_tablet.so`)
// via `cargo-ndk` and place the resulting shared libraries in `app/src/main/jniLibs/`.
val buildRustTabletLib by tasks.registering(Exec::class) {
    group = "rust"
    description = "Compiles kalokorg-tablet Rust cdylib for Android Tablet targets (arm64-v8a, x86_64)"
    workingDir = file("../../..")
    commandLine(
        "cargo",
        "ndk",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-o", file("src/main/jniLibs").absolutePath,
        "build",
        "--release",
        "-p", "kalokorg-tablet"
    )
}
