
native func CustomizeMenu(controller: ref<SingleplayerMenuGameController>);

@wrapMethod(SingleplayerMenuGameController)
private func PopulateMenuItemList() -> Void {
    wrappedMethod();
    CustomizeMenu(this);
}
