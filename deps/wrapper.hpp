#include <RED4ext/RED4ext.hpp>
#include <RED4ext/Scripting/Natives/entEntityID.hpp>
#include <RED4ext/Scripting/Natives/GameTime.hpp>
#include <RED4ext/Scripting/Natives/Generated/EngineTime.hpp>
#include <RED4ext/Scripting/Natives/Generated/red/ResourceReferenceScriptToken.hpp>

namespace versioning
{
    static constexpr uint16_t RUNTIME_INDEPENDENT = -1;
    static constexpr uint8_t SDK_MAJOR = RED4EXT_VER_MAJOR;
    static constexpr uint16_t SDK_MINOR = RED4EXT_VER_MINOR;
    static constexpr uint32_t SDK_PATCH = RED4EXT_VER_PATCH;
    static constexpr uint32_t API_VERSION_LATEST = RED4EXT_API_VERSION_LATEST;
}
