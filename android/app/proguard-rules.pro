# Release is not minified, so these only matter if shrinking is ever turned on:
# SDL reaches its Java glue and our activities by name through JNI/reflection.
-keep class org.libsdl.app.** { *; }
-keep class com.retsend.** { *; }
