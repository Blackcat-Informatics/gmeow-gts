// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

plugins {
    kotlin("jvm") version "2.4.0"
    application
    id("io.gitlab.arturbosch.detekt") version "1.23.8"
}

group = "ca.blackcatinformatics"
version = "0.9.2"

kotlin {
    jvmToolchain(21)
}

application {
    mainClass.set("ca.blackcatinformatics.gts.cli.MainKt")
}

dependencies {
    implementation("com.github.luben:zstd-jni:1.5.7-11")
    implementation("org.bouncycastle:bcprov-jdk18on:1.84")
    implementation("org.bouncycastle:bcpg-jdk18on:1.84")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.11.0")

    testImplementation(kotlin("test"))
}

tasks.test {
    useJUnitPlatform()
}

detekt {
    buildUponDefaultConfig = true
    config.setFrom(files("config/detekt.yml"))
    allRules = false
}
