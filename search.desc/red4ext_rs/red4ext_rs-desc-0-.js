searchState.loadedDescShard("red4ext_rs", 0, "red4ext-rs\nThe RED4ext API version.\nThe author of the plugin.\nA version number representing the RED4ext API version.\nA single class export. This can be used to define a custom …\nA list of exports to register with the game.\nA type representing an empty list of exports.\nA trait for types to be exported to the game.\nA trait for functions that are convertible to pointers. …\nA representation of a function type, including its …\nA trait for types that can be created from a …\nA wrapper around the game application instance.\nA single global function export.\nA trait for functions that can be exported as global …\nA representation of a global function, including its name, …\nA wrapper around function pointers that can be passed to …\nA trait for types that can be converted into a …\nAn error returned when invoking a function fails.\nThe latest version of the RED4ext SDK compatible with this …\nThe latest version of the RED4ext API compatible with this …\nA trait for functions that can be exported as class …\nA representation of a class method, including its name, a …\nThe name of the plugin.\nThe nul terminator character value.\nA trait for types that can be passed across the FFI …\nA definition of a RED4ext plugin.\nInformation about a plugin.\nA set of useful operations that can be performed on a …\nThe version of the game the plugin is compatible with.\nA special version number that indicates the plugin is …\nA trait for types that can be used as the receiver of a …\nA helper struct to set up RTTI registration callbacks.\nThe RTTI system containing information about all types in …\nThe RTTI system containing information about all types in …\nA version number representing the game’s version.\nThe RED4ext SDK version the plugin was built with.\nA handle to the RED4ext SDK environment. This struct …\nA version number representing the RED4ext SDK version.\nA version number in the semantic versioning format.\nA callback function to be called when a state is entered, …\nA listener for state changes in the game application. The …\nAn enum representing different types of game states.\nC-style 16-bit wide string slice for <code>U16CString</code>.\nThe version of the plugin.\nAdd a new RTTI registration callback.\nAdds a listener to a specific state type. The listener …\nReturns a mutable raw pointer to the string.\nReturns the two unsafe mutable pointers spanning the …\nConverts to a mutable slice of the underlying elements.\nReturns a mutable wide string slice to this wide C string …\nReturns a raw pointer to the string.\nReturns the two raw pointers spanning the string slice.\nConverts to a slice of the underlying elements.\nConverts to a slice of the underlying elements, including …\nReturns a wide string slice to this wide C string slice.\nReturns a wide string slice to this wide C string slice.\nAttaches a hook to a target function. The hook will be …\nA macro for conveniently calling functions and methods. If …\nReturns an iterator over the chars of a string slice, and …\nReturns a lossy iterator over the chars of a string slice, …\nReturns an iterator over the <code>char</code>s of a string slice.\nReturns a lossy iterator over the <code>char</code>s of a string slice.\nLogs a message at the debug level. You should generally …\nLogs a message at the debug level.\nDetaches a hook from a target function.\nReturns an object that implements <code>Display</code> for printing …\nRetrieves a statically initialized reference to the plugin …\nLogs a message at the error level. You should generally …\nLogs a message at the error level.\nExports a set of necessary DLL entry points for RED4ext to …\nA list of definitions to be exported automatically when …\nDefine a list of exports to register with the game.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nConstructs a wide C string slice from a pointer and a …\nConstructs a mutable wide C string slice from a mutable …\nConstructs a wide C string slice from a nul-terminated …\nConstructs a mutable wide C string slice from a mutable …\nConstructs a wide C string slice from a pointer and a …\nConstructs a mutable wide C string slice from a mutable …\nConstructs a wide C string slice from a pointer and a …\nConstructs a mutable wide C string slice from a mutable …\nConstructs a wide C string slice from a slice of values …\nConstructs a mutable wide C string slice from a mutable …\nConstructs a wide C string slice from a slice of values, …\nConstructs a mutable wide C string slice from a mutable …\nConstructs a wide C string slice from a slice of values …\nConstructs a mutable wide C string slice from a mutable …\nAcquire a read lock on the RTTI system.\nAcquire a write lock on the RTTI system. You should be …\nReturns a subslice of the string.\nRetrieve a bitfield by its name.\nRetrieve all bitfields and collect them into a <code>RedArray</code>`.\nRetrieve a class by its name.\nRetrieve a mutable reference to a class by its name\nRetrieve a class by its script name.\nRetrieve all instance methods and collect them into a …\nRetrieve base class and its inheritors, optionally …\nRetrieve derived classes, omitting base in the output.\nRetrieve an enum by its name.\nRetrieve an enum by its script name.\nRetrieve all enums and collect them into a <code>RedArray</code>`.\nRetrieve a function by its name.\nRetrieve all global functions and collect them into a …\nReturns a mutable subslice of the string.\nRetrieve all native types and collect them into a <code>RedArray</code>`…\nRetrieve a type by its name.\nReturns an unchecked subslice of the string.\nReturns aa mutable, unchecked subslice of the string.\nA macro for defining global functions. Usually used in …\nHashes of known function addresses.\nDefines a set of hooks that can be attached to target …\nLogs a message at the info level. You should generally use …\nLogs a message at the info level.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nConverts a boxed wide C string slice into an wide C string …\nReturns whether this string contains no data (i.e. is only …\nReturns the length of the string as number of elements (<strong>not</strong>…\nConvenience logging macros. By default all macros require …\nA macro for defining class methods. Usually used in …\nDefine a list of methods to register with the game. …\nRetrieve a reference to a map of all native to script name …\nCreate a new <code>ExportList</code> with the given head and tail.\nCoerces a value into a wide C string slice.\nCreates a new semantic version.\nA function that is called when the plugin is initialized.\nRegister a new <code>ClassHandle</code> with the RTTI system. The …\nRegister a new <code>GlobalFunction</code> with the RTTI system. The …\nCreates a new owned string by repeating this string <code>n</code> …\nshortcut for ResRef creation.\nRetrieve a reference to a map of all script to native name …\nDivide one string slice into two at an index.\nDivide one mutable string slice into two at an index.\nCopys a string to an owned <code>OsString</code>.\nConverts this metadata into a <code>GlobalFunction</code> instance, …\nConverts this metadata into a <code>Method</code> instance, which can …\nCopies the string to a <code>String</code> if it contains valid UTF-16 …\nDecodes the string reference to a <code>String</code> even if it is …\nCopies the string reference to a new owned wide C string.\nCopies the string reference to a new owned wide string.\nLogs a message at the trace level. You should generally …\nLogs a message at the trace level.\nA module encapsulating various types defined in the …\nRetrieve a reference to a map of all types by name.\nLogs a message at the warn level. You should generally use …\nLogs a message at the warn level.\nAlias for <code>u16cstr</code> or <code>u32cstr</code> macros depending on platform. …\nConfigures this method as an event handler (called <code>cb</code> in …\nConfigures this method as final (cannot be overridden).\nSets a callback to be called when the state is entered.\nSets a callback to be called when the state is exited.\nSets a callback to be called when the state is updated.\nResolves a hash to an address.\nLogs a message at the debug level.\nLogs a message at the error level.\nLogs a message at the info level.\nLogs a message at the trace level.\nLogs a message at the warn level.\nA hash representing an immutable string stored in a global …\nClass handle to be used to register a class with …\nA trait for distinguishing between native and scripted …\nsee gameEItemIDFlag and CET initialization.\nAn interface for allocating and freeing memory.\nA trait for types that correspond to bytecode instructions.\nA marker type for native classes. Native classes are …\nA function pointer type for bytecode opcode handlers.\nA reference to a value stored in a pool.\nA trait for types that can be stored in a pool.\nA trait with operations for types that can be stored in a …\nA dynamically sized array.\nA dynamically allocated string.\nA strong reference to a script class. A live instance of …\nA trait for types that represent script classes.\nA trait for operations on script classes.\nA reference to local script data.\nA marker type for scripted classes. Scripted classes are …\nA stack argument to be passed to a function.\nA script stack frame.\nA weak reference to a script class. Before use, it must be …\nPanics\nPanics\nAllocates memory for <code>Self</code>. The resulting value must be …\nAllocates <code>size</code> bytes of memory with <code>alignment</code> bytes …\nReturns the string representation of the <code>CName</code>.\nReturns the number of elements the array can hold without …\nAttempts to cast the reference to a reference of another …\nClears the array, removing all values.\nReturns the context of the stack frame, the <code>this</code> pointer.\nConverts the reference to a <code>WeakRef</code>. This will decrement …\nReturns a reference to the fields of the class.\nReturns a mutable reference to the fields of the class.\nFrees memory pointed by <code>ptr</code>.\nFrees the memory pointed by <code>memory</code>.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the current function of the stack frame.\nRetrieves the next argument from the stack frame.\nReturns <code>true</code> if the stack frame has a code block.\nReturns the type of the value being referenced.\nReturns a reference to the instance of the class.\nInterprets the code at specified offset as an instruction …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns whether the reference is defined.\nReturns <code>true</code> if the array contains no elements.\nReturns an iterator over the elements of the array.\nReturns the number of elements in the array.\nReturns the memory address where local variables are …\nCreates a new empty <code>RedArray</code>.\nCreates a new reference to the class.\nCreates a new empty string.\nCreates a new reference pointing to the provided value.\nCreates a new <code>CName</code> from the given string. This function …\nCreates a new stack argument from a reference to a value.\nCreates a new native class with the given base type. …\nCreates a new reference to the class.\nCreates a new reference to the class and initializes it …\nCreates a new reference to the class and initializes it …\nReturns the memory address where parameters are stored.\nReturns the parent stack frame.\nReturns an iterator over all parent stack frames.\nAdds an element to the end of the array.\nReserve capacity for at least <code>additional</code> more elements to …\nSteps over a single opcode (1 byte).\nPanics\nPanics\nReturns the type of the stack argument.\nReturns a <code>CName</code> representing an undefined name.\nAttempts to upgrade the weak reference to a strong …\nReturns the value being referenced.\nCreates a new empty <code>RedArray</code> with the specified capacity.")