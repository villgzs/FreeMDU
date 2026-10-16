```
git remote add felix-albrecht https://github.com/felix-albrecht/FreeMDU.git
```

```
git remote add bubenkoff https://github.com/bubenkoff/FreeMDU.git
```

```
git remote add BareMetalIoT https://github.com/BareMetalIoT/FreeMDU.git
```

```
git remote add jalr https://github.com/jalr/FreeMDU.git
```

```
git remote add lk-ek https://github.com/lk-ek/FreeMDU.git
```

```
git remote add mauritsvdvijgh https://github.com/mauritsvdvijgh/FreeMDU.git
```

```
git remote add rchl https://github.com/rchl/FreeMDU.git
```

```
git remote add Sn0w3y https://github.com/Sn0w3y/FreeMDU-GUI.git
```

```
git fetch felix-albrecht
```

```
git merge felix-albrecht/protocol-features
```

```
git push origin protocol-features
```
---

# Létrehoz egy új, saját nevű helyi ágat a távoli ág alapján
```
git switch -c sajat-esphome-nev bubenkoff/esphome
```

---
```
#!/bin/bash

# A távoli ágak listája
branches=(
  "remotes/BareMetalIoT/add-id132"
  "remotes/BareMetalIoT/add-id1998"
  "remotes/BareMetalIoT/add-id2895"
  "remotes/BareMetalIoT/master"
  "remotes/Sn0w3y/free-mdu-gui"
  "remotes/Sn0w3y/master"
  "remotes/bubenkoff/adapter-guide"
  "remotes/bubenkoff/add-id132"
  "remotes/bubenkoff/add-id1998"
  "remotes/bubenkoff/add-id457"
  "remotes/bubenkoff/esphome"
  "remotes/bubenkoff/esphome-id132"
  "remotes/bubenkoff/id457-dumps"
  "remotes/bubenkoff/master"
  "remotes/bubenkoff/xiao-c6-antenna"
  "remotes/felix-albrecht/add-id132"
  "remotes/felix-albrecht/add-id1998"
  "remotes/felix-albrecht/add-id324"
  "remotes/felix-albrecht/master"
  "remotes/felix-albrecht/protocol-features"
  "remotes/jalr/add-id472"
  "remotes/jalr/master"
  "remotes/lk-ek/accelerometer-and-w307"
  "remotes/lk-ek/id410-support"
  "remotes/lk-ek/master"
  "remotes/lk-ek/schematic-pcb"
  "remotes/lk-ek/t4223c-dryer"
  "remotes/lk-ek/t4223c-readonly"
  "remotes/mauritsvdvijgh/feature/add-id-2645"
  "remotes/mauritsvdvijgh/master"
  "remotes/origin/HEAD -> origin/master"
  "remotes/origin/add-id132"
  "remotes/origin/add-id1998"
  "remotes/origin/add-id324"
  "remotes/origin/master"
  "remotes/origin/protocol-features"
  "remotes/rchl/feat/new-device"
  "remotes/rchl/feat/wled"
  "remotes/rchl/master"
  "remotes/rchl/protocol-features"
)

for remote_branch in "${branches[@]}"; do
    # 1. Kihagyjuk az 'origin' sorokat és a HEAD mutatókat
    if [[ "$remote_branch" =~ origin ]] || [[ "$remote_branch" =~ "->" ]]; then
        continue
    fi

    # 2. Eltávolítjuk a 'remotes/' prefixet az elejéről
    clean_name="${remote_branch#remotes/}"

    # 3. Az összes '/' jelet átalakítjuk '-' jellé
    local_name="${clean_name//\//-}"

    echo "Létrehozás: $local_name a(z) $remote_branch ágból..."

    # 4. Helyi ág létrehozása a távoli alapján (ha már létezik, a Git jelezni fog)
    git switch -c "$local_name" "$remote_branch"
done

echo "Kész! Az összes ág feldolgozva."
```
