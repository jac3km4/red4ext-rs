// red4ext-rs on macOS: the SDK stand-ins every Mac plugin carries (the memory vault, the CClass virtual slots, the RTTI async id and the
// serializable counter). The Mac SDK header table (RED4ext.SDK-macos, include/RED4ext/Relocation-inl.hpp) routes nine hashes to
// plugin-provided symbols called tweakxl_*, because the game has no Memory::Vault global and no matching RTTI id counters.
//
// Modelled on ltbf/let_there_be_flight/macos/MacVault.cpp (LTBF R41 / R42c, itself from Codeware and TweakXL), trimmed to the seven
// pools the SDK containers and the RTTI allocators reach (PoolRTTI, PoolRTTIFunction, PoolRTTIProperty, PoolScript, PoolEngine,
// PoolRefCount, PoolDefault). Symbols are hidden so each plugin dylib binds its own copy (the id bands below are per plugin).
//
// How it works: a RED4ext::Memory::Vault object is built here with one registry node per pool (handle = FNV1a32 of the pool name,
// the same key the SDK's TPoolInfo::Get and red4ext-rs's vault_get use). The node's PoolStorage::allocatorStorage holds a small tag
// instead of a real storage pointer; tweakxl_vault_alloc and friends read the tag and call the game's own exported
// red::memory::PoolStorageProxy<Pool...>::Allocate / AllocateAligned / Reallocate / ReallocateAligned / Free (dlsym on the main image).
// A block comes back in x0/x1 on arm64 (a 16 byte trivially copyable struct), which is why GameBlock is returned by value.
#include <RED4ext/RED4ext.hpp>
#include <RED4ext/Hashing/FNV1a.hpp>
#include <RED4ext/Memory/Allocators.hpp>
#include <RED4ext/Memory/Vault.hpp>
#include <RED4ext/RTTISystem.hpp>

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <mutex>

#include <dlfcn.h>
#include <mach-o/dyld.h>
#include <mach-o/loader.h>

#define R4E_DIAG(...)                                                \
    do                                                               \
    {                                                                \
        static const bool s_diag = std::getenv("R4E_MAC_DIAG") != nullptr; \
        if (s_diag)                                                  \
        {                                                            \
            std::fprintf(stderr, __VA_ARGS__);                       \
        }                                                            \
    } while (0)

// Local stand-in for the RTTI async id counter. Bands in use: TweakXL 0x7FFF0000, Codeware 0x7FFE0000, ArchiveXL 0x7FFD0000, LTBF
// 0x7FFC0000. red4ext-rs plugins take 0x7FFB0000; two red4ext-rs plugins in one process would share the band (open point in the report).
extern "C" __attribute__((visibility("hidden"))) uint32_t tweakxl_rtti_async_id = 0x7FFB0000;

// Local stand-in for the serializable counter. TweakXL 1<<62, Codeware +1<<61, ArchiveXL +1<<60, LTBF +1<<59; red4ext-rs one band
// further (+1<<58).
extern "C" __attribute__((visibility("hidden"))) volatile int64_t tweakxl_serializable_counter =
    (1LL << 62) + (1LL << 61) + (1LL << 60) + (1LL << 59) + (1LL << 58);

namespace
{
// Tags stored in PoolStorage::allocatorStorage (the SDK masks the low three bits off, so keep them clear).
constexpr uint64_t kTagRtti = 0x10;
constexpr uint64_t kTagFunction = 0x20;
constexpr uint64_t kTagProperty = 0x30;
constexpr uint64_t kTagScript = 0x40;
constexpr uint64_t kTagEngine = 0x50;
constexpr uint64_t kTagRefCount = 0x60;
constexpr uint64_t kTagDefault = 0x70;

// The block the game's pool functions return: two words, so x0 and x1.
struct GameBlock
{
    void* memory;
    uint64_t size;
};

struct PoolOps
{
    const char* name; // Itanium mangled "<length><name>", used to build the exported symbol names
    uint64_t tag;
    uintptr_t expect[5]; // Mac RVAs of the five exports (nm on the game executable), a sanity check only
    GameBlock (*alloc)(uint64_t);
    GameBlock (*allocAligned)(uint64_t, uint32_t);
    GameBlock (*realloc)(GameBlock&, uint64_t);
    GameBlock (*reallocAligned)(GameBlock&, uint64_t, uint32_t);
    void (*freeFn)(GameBlock&);
};

// Evidence for the RVAs: LTBF MacVault.cpp (checked with nm there). The five symbols per pool are
// _ZN3red6memory16PoolStorageProxyINS_<name>EE{8AllocateEy, 15AllocateAlignedEyj, 10ReallocateERNS0_5BlockEy,
// 17ReallocateAlignedERNS0_5BlockEyj, 4FreeERNS0_5BlockE}.
PoolOps g_pools[] = {
    {"8PoolRTTI", kTagRtti, {0x10A1FA4, 0x10A20C0, 0x10A21EC, 0x10A2324, 0x10A2408}, nullptr, nullptr, nullptr, nullptr, nullptr},
    {"16PoolRTTIFunction", kTagFunction, {0x10A268C, 0x10A27A8, 0x10A28D4, 0x10A2A0C, 0x10A2AF0}, nullptr, nullptr, nullptr, nullptr, nullptr},
    {"16PoolRTTIProperty", kTagProperty, {0x10A2D74, 0x10A2E90, 0x10A2FBC, 0x10A30F4, 0x10A31D8}, nullptr, nullptr, nullptr, nullptr, nullptr},
    {"10PoolScript", kTagScript, {0x4E33A00, 0x4E33B1C, 0x4E33C48, 0x4E33D80, 0x4E33E64}, nullptr, nullptr, nullptr, nullptr, nullptr},
    {"10PoolEngine", kTagEngine, {0x1F7D4, 0x1F8F0, 0x1FA1C, 0x1FB54, 0x1FC38}, nullptr, nullptr, nullptr, nullptr, nullptr},
    {"12PoolRefCount", kTagRefCount, {0x1FEBC, 0x1FFD8, 0x20104, 0x2023C, 0x20320}, nullptr, nullptr, nullptr, nullptr, nullptr},
    {"11PoolDefault", kTagDefault, {0x222DC, 0x224A4, 0x22674, 0x22848, 0x2292C}, nullptr, nullptr, nullptr, nullptr, nullptr},
};

const char* kPoolSuffixes[5] = {
    "8AllocateEy",
    "15AllocateAlignedEyj",
    "10ReallocateERNS0_5BlockEy",
    "17ReallocateAlignedERNS0_5BlockEyj",
    "4FreeERNS0_5BlockE",
};

RED4ext::Memory::Vault g_vault;
RED4ext::Memory::PoolStorage g_storage[7];
std::once_flag g_vaultOnce;

uintptr_t GameBase()
{
    const uint32_t count = _dyld_image_count();
    for (uint32_t i = 0; i < count; ++i)
    {
        const auto* header = reinterpret_cast<const mach_header_64*>(_dyld_get_image_header(i));
        if (header && header->filetype == MH_EXECUTE)
        {
            return reinterpret_cast<uintptr_t>(header);
        }
    }
    return reinterpret_cast<uintptr_t>(_dyld_get_image_header(0));
}

void FillPool(uint32_t aIndex, const char* aName, RED4ext::Memory::PoolStorage& aStorage, uint64_t aTag)
{
    auto& node = g_vault.poolRegistry.nodes[aIndex];
    std::memset(&node, 0, sizeof(node));
    node.handle = RED4ext::FNV1a32(aName);
    node.storage = &aStorage;
    aStorage.allocatorStorage = aTag;
    std::strncpy(node.name, aName, sizeof(node.name) - 1);
}

void InitVaultOnce()
{
    std::memset(&g_vault, 0, sizeof(g_vault));
    std::memset(g_storage, 0, sizeof(g_storage));
    FillPool(0, "PoolRTTI", g_storage[0], kTagRtti);
    FillPool(1, "PoolRTTIFunction", g_storage[1], kTagFunction);
    FillPool(2, "PoolRTTIProperty", g_storage[2], kTagProperty);
    FillPool(3, "PoolScript", g_storage[3], kTagScript);
    FillPool(4, "PoolEngine", g_storage[4], kTagEngine);
    FillPool(5, "PoolRefCount", g_storage[5], kTagRefCount);
    FillPool(6, "PoolDefault", g_storage[6], kTagDefault);

    const uintptr_t base = GameBase();
    bool allOk = true;
    for (auto& pool : g_pools)
    {
        void* slots[5] = {};
        for (int i = 0; i < 5; ++i)
        {
            char symbol[160];
            std::snprintf(symbol, sizeof(symbol), "_ZN3red6memory16PoolStorageProxyINS_%sEE%s", pool.name, kPoolSuffixes[i]);
            // dlsym takes the Itanium name with one leading underscore fewer than dyld_info prints; RTLD_MAIN_ONLY searches the game.
            void* found = dlsym(RTLD_MAIN_ONLY, symbol);
            const auto rva = found ? reinterpret_cast<uintptr_t>(found) - base : 0;
            R4E_DIAG("[r4e vault] %s %s rva=%llx expected=%llx\n", pool.name, kPoolSuffixes[i], static_cast<unsigned long long>(rva),
                     static_cast<unsigned long long>(pool.expect[i]));
            if (!found || rva != pool.expect[i])
            {
                std::fprintf(stderr, "[r4e vault] MISMATCH %s %s\n", pool.name, kPoolSuffixes[i]);
                allOk = false;
                slots[i] = nullptr;
            }
            else
            {
                slots[i] = found;
            }
        }
        pool.alloc = reinterpret_cast<GameBlock (*)(uint64_t)>(slots[0]);
        pool.allocAligned = reinterpret_cast<GameBlock (*)(uint64_t, uint32_t)>(slots[1]);
        pool.realloc = reinterpret_cast<GameBlock (*)(GameBlock&, uint64_t)>(slots[2]);
        pool.reallocAligned = reinterpret_cast<GameBlock (*)(GameBlock&, uint64_t, uint32_t)>(slots[3]);
        pool.freeFn = reinterpret_cast<void (*)(GameBlock&)>(slots[4]);
    }
    R4E_DIAG("[r4e vault] ok=%d\n", allOk ? 1 : 0);
}

void InitVault()
{
    std::call_once(g_vaultOnce, InitVaultOnce);
}

PoolOps* PoolFor(uintptr_t aTag)
{
    for (auto& pool : g_pools)
    {
        if (pool.tag == aTag)
        {
            return &pool;
        }
    }
    std::fprintf(stderr, "[r4e vault] unknown tag 0x%llx\n", static_cast<unsigned long long>(aTag));
    return nullptr;
}

bool AddressInImage(uint32_t aIndex, uintptr_t aAddr)
{
    const auto* header = reinterpret_cast<const mach_header_64*>(_dyld_get_image_header(aIndex));
    if (!header)
    {
        return false;
    }
    const intptr_t slide = _dyld_get_image_vmaddr_slide(aIndex);
    auto* cmd = reinterpret_cast<const load_command*>(header + 1);
    for (uint32_t i = 0; i < header->ncmds; ++i)
    {
        if (cmd->cmd == LC_SEGMENT_64)
        {
            const auto* seg = reinterpret_cast<const segment_command_64*>(cmd);
            const auto start = static_cast<uintptr_t>(seg->vmaddr) + static_cast<uintptr_t>(slide);
            if (aAddr >= start && aAddr < start + seg->vmsize)
            {
                return true;
            }
        }
        cmd = reinterpret_cast<const load_command*>(reinterpret_cast<const uint8_t*>(cmd) + cmd->cmdsize);
    }
    return false;
}

int GameImageIndex()
{
    const uint32_t count = _dyld_image_count();
    for (uint32_t i = 0; i < count; ++i)
    {
        const auto* header = reinterpret_cast<const mach_header_64*>(_dyld_get_image_header(i));
        if (header && header->filetype == MH_EXECUTE)
        {
            return static_cast<int>(i);
        }
    }
    return -1;
}
} // namespace

extern "C" __attribute__((visibility("hidden"))) void* tweakxl_memory_vault()
{
    InitVault();
    return &g_vault;
}

extern "C" __attribute__((visibility("hidden"))) void tweakxl_vault_alloc(void* aStorage, void* aResult, uint64_t aSize)
{
    InitVault();
    auto* result = static_cast<RED4ext::Memory::AllocationResult*>(aResult);
    result->memory = nullptr;
    result->size = 0;
    auto* pool = PoolFor(reinterpret_cast<uintptr_t>(aStorage));
    if (!pool || !pool->alloc)
    {
        return;
    }
    const GameBlock block = pool->alloc(aSize);
    result->memory = block.memory;
    result->size = block.memory ? block.size : 0;
}

extern "C" __attribute__((visibility("hidden"))) void tweakxl_vault_alloc_aligned(void* aStorage, void* aResult, uint64_t aSize,
                                                                                    uint32_t aAlign)
{
    InitVault();
    auto* result = static_cast<RED4ext::Memory::AllocationResult*>(aResult);
    result->memory = nullptr;
    result->size = 0;
    auto* pool = PoolFor(reinterpret_cast<uintptr_t>(aStorage));
    if (!pool || !pool->allocAligned)
    {
        return;
    }
    const GameBlock block = pool->allocAligned(aSize, aAlign);
    result->memory = block.memory;
    result->size = block.memory ? block.size : 0;
}

extern "C" __attribute__((visibility("hidden"))) void tweakxl_vault_realloc(void* aStorage, void* aResult, void* aInput, uint64_t aSize)
{
    InitVault();
    auto* result = static_cast<RED4ext::Memory::AllocationResult*>(aResult);
    result->memory = nullptr;
    result->size = 0;
    auto* pool = PoolFor(reinterpret_cast<uintptr_t>(aStorage));
    if (!pool || !pool->realloc || !aInput)
    {
        return;
    }
    const GameBlock block = pool->realloc(*static_cast<GameBlock*>(aInput), aSize);
    result->memory = block.memory;
    result->size = block.memory ? block.size : 0;
}

extern "C" __attribute__((visibility("hidden"))) void tweakxl_vault_realloc_aligned(void* aStorage, void* aResult, void* aInput,
                                                                                      uint64_t aSize, uint32_t aAlign)
{
    InitVault();
    auto* result = static_cast<RED4ext::Memory::AllocationResult*>(aResult);
    result->memory = nullptr;
    result->size = 0;
    auto* pool = PoolFor(reinterpret_cast<uintptr_t>(aStorage));
    if (!pool || !pool->reallocAligned || !aInput)
    {
        return;
    }
    const GameBlock block = pool->reallocAligned(*static_cast<GameBlock*>(aInput), aSize, aAlign);
    result->memory = block.memory;
    result->size = block.memory ? block.size : 0;
}

extern "C" __attribute__((visibility("hidden"))) void tweakxl_vault_free(void* aStorage, void* aInput)
{
    InitVault();
    auto* pool = PoolFor(reinterpret_cast<uintptr_t>(aStorage));
    if (!pool || !pool->freeFn || !aInput)
    {
        return;
    }
    pool->freeFn(*static_cast<GameBlock*>(aInput));
}

// CClass virtual functions the SDK reaches by hash. The Mac offset is the Windows offset plus 8 (D1 and D0 take two slots). The
// functions are read from the live vtable of the game's IScriptable class object, so they are always the game's own; a slot that does
// not point into the game executable (for example one patched by another plugin) is refused and answered with 0.
extern "C" __attribute__((visibility("hidden"))) uintptr_t tweakxl_class_vslot(uint32_t aByteOffset)
{
    static bool ready = false;
    static uintptr_t slots[11] = {};
    static const uint32_t kOffsets[11] = {0x68, 0x70, 0x88, 0x90, 0x98, 0xA0, 0xA8, 0xB8, 0xC8, 0xD0, 0xD8};
    if (!ready)
    {
        auto* rtti = RED4ext::CRTTISystem::Get();
        auto* cls = rtti ? rtti->GetClass(RED4ext::CName("IScriptable")) : nullptr;
        if (!cls)
        {
            std::fprintf(stderr, "[r4e vslot] IScriptable missing\n");
            return 0;
        }
        auto* vt = *reinterpret_cast<uintptr_t**>(cls);
        const int game = GameImageIndex();
        const uintptr_t base = GameBase();
        for (int i = 0; i < 11; ++i)
        {
            const uintptr_t fn = vt[kOffsets[i] / 8];
            const bool inGame = game >= 0 && AddressInImage(static_cast<uint32_t>(game), fn);
            R4E_DIAG("[r4e vslot] off=%x rva=%llx inGame=%d\n", kOffsets[i], static_cast<unsigned long long>(fn - base), inGame ? 1 : 0);
            slots[i] = inGame ? fn : 0;
            if (!inGame)
            {
                std::fprintf(stderr, "[r4e vslot] reject off=%x (not in the game image)\n", kOffsets[i]);
            }
        }
        ready = true;
    }
    for (int i = 0; i < 11; ++i)
    {
        if (kOffsets[i] == aByteOffset)
        {
            return slots[i];
        }
    }
    std::fprintf(stderr, "[r4e vslot] unknown offset %x\n", aByteOffset);
    return 0;
}
