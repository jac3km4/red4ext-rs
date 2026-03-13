namespace Red4extRs
{
struct RTTIRegistrator
{
    using CallbackFunc = void (*)();
    static void Add(CallbackFunc aRegFunc, CallbackFunc aPostRegFunc);
};
}
