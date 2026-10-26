#include "bridge/bridge.hpp"
#include <RED4ext/RED4ext.hpp>
#include <RED4ext/Callback.hpp>
#include <RED4ext/Scripting/Natives/entEntityID.hpp>
#include <RED4ext/Scripting/Natives/GameTime.hpp>
#include <RED4ext/Scripting/Natives/ScriptGameInstance.hpp>
#include <RED4ext/Scripting/Natives/Generated/EngineTime.hpp>
#include <RED4ext/Scripting/Natives/Generated/game/ScriptableSystem.hpp>
#include <RED4ext/Scripting/Natives/Generated/red/ResourceReferenceScriptToken.hpp>

namespace versioning
{
    static constexpr uint16_t RUNTIME_INDEPENDENT = -1;
#if defined(__APPLE__)
    // macOS: the private loader (red4ext/RED4ext.dylib) knows API version 0 only. First in-game boot, red4ext.log:
    //   [RED4ext] [warning] 'R4ersGate' is using an unsupported API version. API version: 1, path: '.../red4ext/plugins/R4ersGate/R4ersGate.dylib'
    // The dylib stayed loaded but Main never ran. So report what the C++ plugins report: RED4EXT_API_VERSION_1_COMPAT_0 (0, Api/ApiVersion.hpp)
    // and RED4EXT_V1_SDK_VERSION_1_0_0_COMPAT_0_5_0 (SDK 0.5.0, Api/v1/Version.hpp). jackmaxwil's loader accepts both (PluginSystem.cpp:
    // MINIMUM_API_VERSION RED4EXT_API_VERSION_1_COMPAT_0, MINIMUM_SDK_VERSION the same 0.5.0 macro). The v0 and v1 structs are identical.
    static constexpr uint8_t SDK_MAJOR = 0;
    static constexpr uint16_t SDK_MINOR = 5;
    static constexpr uint32_t SDK_PATCH = 0;
    static constexpr uint32_t API_VERSION_LATEST = RED4EXT_API_VERSION_1_COMPAT_0;
#else
    static constexpr uint8_t SDK_MAJOR = RED4EXT_VER_MAJOR;
    static constexpr uint16_t SDK_MINOR = RED4EXT_VER_MINOR;
    static constexpr uint32_t SDK_PATCH = RED4EXT_VER_PATCH;
    static constexpr uint32_t API_VERSION_LATEST = RED4EXT_API_VERSION_1;
#endif
}
