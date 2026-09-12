plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.shadelight.iausage"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.shadelight.iausage"
        minSdk = 26
        targetSdk = 36
        // versionName sigue la versión general (tag vX.Y.Z sin la `v`).
        // versionCode debe crecer siempre aunque versionName se repita:
        // en CI se inyecta vía -PIAUSAGE_VERSION_CODE=$GITHUB_RUN_NUMBER.
        // Al publicar una release, subir este default si el número de CI
        // quedase por debajo del último versionCode subido a Play.
        versionCode = providers.gradleProperty("IAUSAGE_VERSION_CODE").orNull?.toIntOrNull() ?: 3
        versionName = providers.gradleProperty("IAUSAGE_VERSION_NAME").orNull ?: "0.3.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    signingConfigs {
        create("release") {
            val storeFilePath = providers.gradleProperty("IAUSAGE_STORE_FILE").orNull
            if (storeFilePath != null) {
                storeFile = file(storeFilePath)
                storePassword = providers.gradleProperty("IAUSAGE_STORE_PASSWORD").orNull
                keyAlias = providers.gradleProperty("IAUSAGE_KEY_ALIAS").orNull
                keyPassword = providers.gradleProperty("IAUSAGE_KEY_PASSWORD").orNull
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            if (providers.gradleProperty("IAUSAGE_STORE_FILE").isPresent) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    buildFeatures { compose = true; buildConfig = true }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2025.06.01")
    implementation(composeBom)
    androidTestImplementation(composeBom)
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.9.1")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.9.1")
    implementation("androidx.work:work-runtime-ktx:2.10.2")
    implementation("androidx.glance:glance-appwidget:1.1.1")
    implementation("com.google.android.gms:play-services-code-scanner:16.1.0")
    implementation("com.goterl:lazysodium-android:5.2.0@aar")
    implementation("net.java.dev.jna:jna:5.17.0@aar")
    implementation("org.bouncycastle:bcprov-jdk18on:1.79")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
}
