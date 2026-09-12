# LazySodium discovers JNI methods at runtime. Keep its public bridge intact.
-keep class com.goterl.lazysodium.** { *; }
-keep class com.sun.jna.** { *; }

# JNA's AWT integration is desktop-only dead code on Android; java.awt.* is
# not on the platform, so just silence R8's missing-class warnings for it.
-dontwarn java.awt.**
