module ForgetArgs

native func OnIncrease(system: ref<System>, key: TweakDBID) -> Void;
native func OnDecrease(system: ref<System>) -> Void;

// Game.GetPlayer():SimulateIncrease(TDBID.Create("Items.FirstAidWhiffV0"));
@addMethod(PlayerPuppet)
public func SimulateIncrease(id: TweakDBID) -> Void {
    let system = System.GetInstance(this.GetGame());
    OnIncrease(system, id);
    LogState(system);
}

// Game.GetPlayer():SimulateDecrease();
@addMethod(PlayerPuppet)
public func SimulateDecrease(id: TweakDBID) -> Void {
    let system = System.GetInstance(this.GetGame());
    OnDecrease(system);
    LogState(system);
}

public func LogState(system: ref<System>) -> Void {
    let items = system.Items();
    let keys = items.Keys();
    let values = items.Values();
    let idx = 0;
    let value: ref<Item>;
    for key in keys {
        value = values[idx];
        LogChannel(n"DEBUG", s"key: \(TDBID.ToStringDEBUG(keys[idx])), value: \(IsDefined(value) ? ToString(value.value) : "undefined")");
        idx += 1;
    }
}

public class Item extends IScriptable {
    private persistent let value: Int32;
    private func Get() -> Int32 { return this.value; }
    private func Set(value: Int32) -> Void { this.value = value; }
}
public class Items extends IScriptable {
    private persistent let keys: array<TweakDBID>;
    private persistent let values: array<ref<Item>>;
    private func Values() -> array<ref<Item>> { return this.values; }
    private func SetValues(values: array<ref<Item>>) -> Void { this.values = values; }
    private func Keys() -> array<TweakDBID> { return this.keys; }
    private func SetKeys(keys: array<TweakDBID>) -> Void  { this.keys = keys; }
    private func Create(value: Int32) -> ref<Item> {
        let item = new Item();
        item.value = value;
        return item;
    }
}
public class System extends ScriptableSystem {
    private persistent let items: ref<Items>;
    func Items() -> ref<Items> {
        return this.items;
    }
    public final static func GetInstance(game: GameInstance) -> ref<System> {
        let container = GameInstance.GetScriptableSystemsContainer(game);
        return container.Get(n"ForgetArgs.System") as System;
    }
    // prevent 'ref was uninitialized' on first run
    private func OnAttach() -> Void {
        if !IsDefined(this.items)
           || NotEquals(ArraySize(this.items.keys), ArraySize(this.items.values)) {
            LogChannel(n"DEBUG", "initializing or resetting corrupted items");
            let items = new Items();
            items.keys = [];
            items.values = [];
            this.items = items;
        }
    }
}