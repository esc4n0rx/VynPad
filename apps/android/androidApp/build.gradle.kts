plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.compose.multiplatform)
}

val ndkVersion = "27.1.12297006"
val androidHome = System.getenv("ANDROID_HOME") ?: System.getenv("ANDROID_SDK_ROOT")
    ?: error("Defina ANDROID_HOME apontando para o Android SDK")

val buildCoreAndroid = tasks.register<CargoNdkBuild>("buildCoreAndroid") {
    workspaceDir.set(rootProject.layout.projectDirectory.dir("../.."))
    abis.set(listOf("arm64-v8a", "armeabi-v7a", "x86_64"))
    ndkDir.set("$androidHome/ndk/$ndkVersion")
    jniLibsDir.set(layout.buildDirectory.dir("rustJniLibs"))
    doNotTrackState("O cargo faz o próprio controle de cache")
}

android {
    ndkVersion = "27.1.12297006"
    namespace = "app.vynpad.android"
    compileSdk = 37

    defaultConfig {
        applicationId = "app.vynpad.android"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
        }
    }

    sourceSets.getByName("main").jniLibs.directories.add(
        layout.buildDirectory.dir("rustJniLibs").get().asFile.path,
    )

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation(project(":shared"))
    implementation(libs.androidx.activity.compose)
}

// A biblioteca nativa precisa existir antes de o AGP juntar as .so no APK.
tasks.configureEach {
    if (name.startsWith("merge") && name.endsWith("JniLibFolders")) dependsOn(buildCoreAndroid)
}
