#include "glue.hpp"
#include <RED4ext/RED4ext.hpp>
#include <RED4ext/RTTISystem.hpp>

void Red4extRs::RTTIRegistrator::Add(CallbackFunc aRegFunc, CallbackFunc aPostRegFunc)
{
    if (aRegFunc)
    {
        RED4ext::CRTTISystem::Get()->AddRegisterCallback(aRegFunc);
    }

    if (aPostRegFunc)
    {
        RED4ext::CRTTISystem::Get()->AddPostRegisterCallback(aPostRegFunc);
    }
}
