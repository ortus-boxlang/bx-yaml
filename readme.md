# ⚡︎ BoxLang Module: BoxLang YAML Support

```
|:------------------------------------------------------:|
| ⚡︎ B o x L a n g ⚡︎
| Dynamic : Modular : Productive
|:------------------------------------------------------:|
```

<blockquote>
	Copyright Since 2023 by Ortus Solutions, Corp
	<br>
	<a href="https://www.boxlang.io">www.boxlang.io</a> |
	<a href="https://www.ortussolutions.com">www.ortussolutions.com</a>
</blockquote>

<p>&nbsp;</p>

## Welcome to the BoxLang YAML Module

This module provides a YAML parser and emitter for BoxLang.  It is based on the [SnakeYAML](https://bitbucket.org/asomov/snakeyaml) library and provides a simple way to parse and emit YAML content in BoxLang.

## Installation

You can install it via the BoxLang CLI:

```bash
install-bx-module bx-yaml
```

Or you can install it into a CommandBox server:

```bash
box install boxlang-yaml
```

## Usage

This module registers the following BIFS:

- `yamlSerialize( content, [filepath], [charset=utf8] ):yaml` : Serialize a BoxLang variable into a YAML string.  You can also serialize to a file if you provide a file path.
- `yamlDeserialize( content ):any` : Deserialize a YAML string into a BoxLang variable.
- `yamlDeserializeFile( filepath, [charset=utf8] ):any` : Deserialize a YAML file into a BoxLang variable.

Here is a simple example:

```java
// Serialize a BoxLang structure into a YAML string
yaml = yamlSerialize( { name="Luis", age=21, city="Orlando" } );
// Serialize a more complex structure
yaml = yamlSerialize( { name="Luis", age=21, city="Orlando", address={ street="1234", city="Orlando", state="FL" } } );

// Deserialize a YAML string into a BoxLang structure
data = yamlDeserialize( yaml );

// Deserialize a YAML file into a BoxLang structure
data = yamlDeserializeFile( "data.yml" );
```

## BoxLang Class Serialization

BoxLang classes will be serialized as a structure according to its properties.  However it must adhere to the following rules:

- The class must have a `serializable = true` annotation. or none at all, that is the default.
- The property must have a `serializable = true` annotation or none at all, that is the default.
- The property must NOT exist in the `yamlExclude` list in the class or the parent class.
- The property must NOT have a `yamlExclude` annotation.

## BoxLang Class Custom Serialization

If you are serializing BoxLang classes, you can implement the `toYAML()` method in your classes to provide a custom serialization.  Here is an example:

```java
class {

	function init( required name="", required age=0, required city="" ){
		variables.name = arguments.name;
		variables.age = arguments.age;
		variables.city = arguments.city;
		return this;
	}

	function toYAML(){
		return { name=variables.name, city=variables.city } );
	}

}
```

## MatchBox (no JVM)

The same package also runs on [MatchBox](https://github.com/ortus-boxlang/matchbox), the BoxLang VM for native binaries, WebAssembly and small devices. `box.json` lists a prebuilt native library per target under `matchbox.artifacts`, so `matchbox --module bx-yaml app.bxs` and `matchbox --target native` work without a JVM, Rust or Cargo. The JVM loader ignores that metadata and uses the jar.

Libraries are published for `x86_64` and `aarch64` on Linux (glibc 2.28 or newer), macOS and Windows. MatchBox itself has validated plugin execution on Linux only, so the macOS and Windows libraries are built and tested in CI but are not yet blocking there.

`yamlDeserialize()`, `yamlDeserializeFile()` and `yamlSerialize()` take the same arguments and return the same kinds of values on both backends: mappings become structs (case-insensitive keys), sequences arrays, plain dates and timestamps dates. Anchors, aliases, merge keys (`<<`) and block and flow scalars work, and a document with duplicate keys, several documents or a null key is refused, as SnakeYAML refuses them. One contract script, `native/yaml-contract.bxs`, runs against the JVM and MatchBox. These things differ:

- **YAML 1.2 rules.** The native reader resolves plain scalars by the YAML 1.2 core schema; SnakeYAML uses 1.1. So `yes`, `no`, `on` and `off` are strings (SnakeYAML: booleans), `017` is the integer 17 (15), `1_000`, `1:30` and `0b101` are strings (1000, 90 and 5), and `0o17` is the integer 15 (a string). Quote or spell out values that must read the same on both.
- **Tags.** `!!str`, `!!int`, `!!float`, `!!bool`, `!!null` and `!!timestamp` work. `!!binary`, `!!set` and custom tags are refused.
- **Order.** Mappings keep the order of the document. On the JVM their order is not defined.
- **Writing.** Whole numbers are written as integers (`{ n: 2.0 }` writes `n: 2`), since MatchBox does not keep integers and decimals apart. An empty sequence or mapping is `[]` or `{}`, where SnakeYAML writes `[` and `]` on two lines. A float outside `0.0001` to `10^16` is written like `1.0e+300`.
- **What can be written.** Structs, arrays, strings, numbers, booleans, null and dates. BoxLang class instances (and their `toYaml()` and `yamlExclude` handling), queries and functions are only written by the JVM backend.
- **Errors.** File problems are the same `Error deserializing yaml` and `Error serializing yaml` exceptions. A syntax error is a generic exception on MatchBox, with the parser's own wording.
- **UTF-8 only.** A `charset` other than UTF-8 fails.

Build and test it with a Rust toolchain:

```bash
cargo test --manifest-path native/Cargo.toml            # the reader, the writer, and a round trip through both
./gradlew downloadBoxLang
./gradlew build -x test -PmatchboxTargets=x86_64-unknown-linux-gnu   # adds the library to build/module
matchbox --module build/module native/yaml-contract.bxs
java -cp src/test/resources/libs/boxlang-*.jar native/YamlContract.java build/module native/yaml-contract.bxs
```

## Ortus Sponsors

BoxLang is a professional open-source project and it is completely funded by the [community](https://patreon.com/ortussolutions) and [Ortus Solutions, Corp](https://www.ortussolutions.com).  Ortus Patreons get many benefits like a cfcasts account, a FORGEBOX Pro account and so much more.  If you are interested in becoming a sponsor, please visit our patronage page: [https://patreon.com/ortussolutions](https://patreon.com/ortussolutions)

### THE DAILY BREAD

 > "I am the way, and the truth, and the life; no one comes to the Father, but by me (JESUS)" Jn 14:1-12
