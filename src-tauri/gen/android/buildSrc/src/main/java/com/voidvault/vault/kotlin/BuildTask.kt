import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.TaskAction

abstract class BuildTask : DefaultTask() {
    @Input
    var rootDirRel: String? = null
    @Input
    var projectDir: String? = null
    @Input
    var target: String? = null
    @Input
    var release: Boolean? = null

    @TaskAction
    fun assemble() {
        val projectDir = File(projectDir ?: throw GradleException("projectDir cannot be null"))
        val rootDirRel = rootDirRel ?: throw GradleException("rootDirRel cannot be null")
        val target = target ?: throw GradleException("target cannot be null")
        val release = release ?: throw GradleException("release cannot be null")

        val triple = when (target) {
            "aarch64" -> "aarch64-linux-android"
            "armv7" -> "armv7-linux-androideabi"
            "i686" -> "i686-linux-android"
            "x86_64" -> "x86_64-linux-android"
            else -> throw GradleException("Unsupported Rust target: $target")
        }
        val abi = when (target) {
            "aarch64" -> "arm64-v8a"
            "armv7" -> "armeabi-v7a"
            "i686" -> "x86"
            "x86_64" -> "x86_64"
            else -> throw GradleException("Unsupported Rust target: $target")
        }
        val profile = if (release) "release" else "debug"

        val source = File(projectDir, rootDirRel)
            .resolve("target/$triple/$profile/libvoidvault_lib.so")
        if (!source.isFile) {
            throw GradleException(
                "Missing Rust library at ${source.absolutePath}. " +
                    "Build it with: cargo build --target $triple${if (release) " --release" else ""}"
            )
        }

        val destinationDir = File(projectDir, "src/main/jniLibs/$abi")
        destinationDir.mkdirs()
        val destination = File(destinationDir, "libvoidvault_lib.so")
        Files.copy(source.toPath(), destination.toPath(), StandardCopyOption.REPLACE_EXISTING)
        logger.lifecycle("Copied ${source.absolutePath} to ${destination.absolutePath}")
    }
}
