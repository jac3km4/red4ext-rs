#include <RED4ext/RED4ext.hpp>
#include <RED4ext/Scripting/Natives/entEntityID.hpp>

namespace versioning
{
    enum constants
    {
        RUNTIME_INDEPENDENT = -1,
        SDK_MAJOR = RED4EXT_VER_MAJOR,
        SDK_MINOR = RED4EXT_VER_MINOR,
        SDK_PATCH = RED4EXT_VER_PATCH,
        API_VERSION_LATEST = RED4EXT_API_VERSION_LATEST
    };
}
