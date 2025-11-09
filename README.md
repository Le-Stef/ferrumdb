# FerrumDB

**Une base de données clefs/valeurs en mémoire, super rapide, compatible avec Redis, écrite en Rust**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

FerrumDB est une base de données clefs/valeurs en mémoire. Léger, rapide et compatible avec Redis, il est développé en Rust. Il implémente le protocole RESP2 et offre d'excellentes performances grâce à son sharding intégré et à la persistance AOF.

## Fonctionnalités

- ✅ **Protocole RESP2** - Compatibilité totale avec les clients Redis (RESP2)
- ✅ **Sharding intégré** - Répartition automatique des clés entre les cœurs du processeur (jusqu'à 16 shards)
- ✅ **Persistance AOF** - Fichier en ajout seul avec sommes de contrôle, algorithme de hashage 64 bits 'xxhash64'
- ✅ **Tableau de bord Web** - Surveillance en temps réel sur le port 8080
- ✅ **Aucune configuration requise** - Fonctionne dès l'installation
- ✅ **Multiplateforme** - Windows natif (WSL inutile !), Linux (y compris Raspberry Pi), macOS
- ✅ **Léger** - Binaire unique, dépendances minimales

## Démarrage rapide

### Installation

```bash
# Cloner le dépôt
git clone https://github.com/yourusername/ferrumdb.git
cd ferrumdb

# Créer une version finale
cargo build --release

# Exécuter
cargo run --release
```

## Usage

### Démarrage du serveur

```bash
# Ports par défaut : 6379 (protocole Redis), 8080 (Interface Web)
cargo run --release
```

Le serveur va :
- Détecter les cœurs du processeur et créer un nombre optimal de shards (max. 16)
- Écouter sur `127.0.0.1:6379` pour le protocole Redis
- Proposer le tableau de bord web sur `http://127.0.0.1:8080` : à utiliser depuis votre navigateur
- Créer des fichiers AOF pour chaque shard (`ferrumdb_shard_*.aof`)

### Connexion avec les clients Redis

```bash
# Utilisation de redis-cli
redis-cli -h 127.0.0.1 -p 6379

# Exemple Python
import redis
r = redis.Redis(host='127.0.0.1', port=6379)
r.set('key', 'value')
print(r.get('key'))  # b'value'
```

### Tableau de bord Web

Ouvrez `http://127.0.0.1:8080` dans votre navigateur pour accéder à :
- Statistiques système en temps réel (CPU, mémoire)
- Indicateurs au niveau des shards (clés, mémoire, distribution)
- Console de commande interactive
- Surveillance des performances

## Commandes prises en charge

### Chaînes (2 commandes)
- `GET`, `SET`

### Clefs (2 commandes)
- `DEL`, `EXISTS`

### TTL (2 commandes)
- `EXPIRE`, `TTL`

### Compteurs (4 commandes)
- `INCR`, `INCRBY`, `DECR`, `DECRBY`

### Listes (4 commandes)
- `LPUSH`, `RPUSH`, `LRANGE`, `LLEN`

### Sets (3 commandes)
- `SADD`, `SMEMBERS`, `SCARD`

### Hashes (6 commandes)
- `HSET`, `HGET`, `HGETALL`, `HDEL`, `HKEYS`, `HINCRBY`

### Administration (4 commandes)
- `INFO`, `FLUSHDB`, `KEYS`, `CLIENT`

**Total: 28 commandes implementées**

## 🏗️ Architecture

FerrumDB utilise une **architecture fragmentée en 'shards'** pour des performances optimales :

```
┌─────────────────────────────────────────────────────────┐
│                    Connexions Client                    │
│                   (Runtime asynchrone Tokio)            │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
              ┌───────────────┐
              │ Parseur RESP2 │
              └──────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │ Routeur Cluster │  ← distribution des clefs via SipHash13
            │   (SipHash13)   │
            └────┬────────────┘
                 │
        ┌────────┴────────┬───────────┬──────────┐
        ▼                 ▼           ▼          ▼
    ┌────────┐       ┌────────┐  ┌────────┐  ┌────────┐
    │ Thread │       │ Thread │  │ Thread │  │ Thread │
    │Shard 0 │       │Shard 1 │  │Shard 2 │  │  ... N │
    ├────────┤       ├────────┤  ├────────┤  ├────────┤
    │Stockage│       │Stockage│  │Stockage│  │Stockage│
    ├────────┤       ├────────┤  ├────────┤  ├────────┤
    │  AOF   │       │  AOF   │  │  AOF   │  │  AOF   │
    └────────┘       └────────┘  └────────┘  └────────┘
```

### Décisions clefs en matière de conception

- **Threads par shard** : chaque shard s'exécute dans son propre thread avec une mémoire dédiée
- **Communication sans verrouillage** : canaux MPSC pour la communication entre shards
- **Hachage cohérent** : SipHash13 garantit une distribution uniforme des clefs
- **Persistance isolée** : chaque shard conserve son propre fichier AOF
- **Analyse sans copie** : utilisation de `bytes::Bytes` pour une gestion efficace de la mémoire tampon

## Performance

FerrumDB est conçu pour des opérations à haut débit et faible latence :

- **Latence cible** : < 100 µs par opération
- **Débit cible** : plus de 100 000 opérations/seconde par cœur
- **Efficacité mémoire** : structures de données compactes avec des allocations minimales

### Benchmarking

```bash
# Utilisation de redis-benchmark
redis-benchmark -h 127.0.0.1 -p 6379 -t set,get -n 100000 -q

# Exemple de résultats (peut varier suivant les configurations):
# SET: ~150,000 requests per second
# GET: ~180,000 requests per second
```

## Configuration

Actuellement, FerrumDB fonctionne sans aucune configuration. Les prochaines versions prendront en charge :

- La configuration personnalisée des ports
- Les limites de mémoire et les politiques d'éviction
- Les politiques de synchronisation AOF (toujours, toutes les secondes, jamais)
- La personnalisation du nombre de shards

## Roadmap

- [x] **Phase 1** : Commandes de base (SET, GET, DEL, EXPIRE)
- [x] **Phase 2** : Listes, ensembles, hachages, compteurs, AOF, interface utilisateur Web
- [x] **Phase 3** : Implémentation complète du partitionnement
- [ ] **Phase 4** : Rejouer l'AOF, instantanés RDB, améliorations TTL
- [ ] **Phase 5** : Clustering multi-nœuds (découverte des nœuds)
- [ ] **Phase 6** : Clustering multi-nœuds (distribution des données)
- [ ] **Phase 7** : Compactage AOF en arrière-plan
- [ ] **Phase 8** : Optimisation des performances

## Developpement

### Compilation à partir du code source

```bash
# build de debug
cargo build

# build de release (optimisé)
cargo build --release

# Lancement des tests
cargo test
```

### Structure du projet

```
ferrumdb/
├── src/
│   ├── protocol/       # Parseur et encodeur RESP2
│   ├── server/         # Couche réseau (Tokio)
│   ├── cluster/        # Sharding et routage
│   ├── commands/       # Implémentation des commandes
│   ├── store/          # Structures de données en mémoire
│   ├── aof/            # Persistance AOF
│   ├── web/            # Tableau de bord Web
│   └── main.rs         # Point d'entrée
├── Cargo.toml
```

### Directives de développement

- Écrire des tests pour les nouvelles fonctionnalités.
- Suivre les idiomes et les meilleures pratiques Rust.
- Documenter les API publiques.
- Garder les commits atomiques et bien décrits.

### Signaler des problèmes

Veuillez utiliser GitHub Issues pour signaler des bogues ou demander des fonctionnalités. Indiquez :
- La version de FerrumDB.
- Le système d'exploitation.
- Les étapes pour reproduire le problème.
- Le comportement attendu par rapport au comportement réel.

## Limitations connues

- Pas de RESP3 pour le moment
- La relecture AOF n'est pas encore implémentée (les données sont chargées mais ne sont pas appliquées au démarrage)
- Maximum de 16 shards (sera configurable dans les prochaines versions)
- Pas encore de prise en charge pub/sub
- Pas de prise en charge des transactions (MULTI/EXEC)
- Pas de scripting
- Pas de mode cluster (nœud unique multi-shard uniquement)

## Exemples

### Python (redis-py)

```python
import redis

r = redis.Redis(host='127.0.0.1', port=6379, decode_responses=True)

# Chaînes
r.set('name', 'FerrumDB')
print(r.get('name'))  # 'FerrumDB'

# Compteurs
r.incr('visits')
r.incrby('visits', 10)
print(r.get('visits'))  # '11'

# Listes
r.lpush('tasks', 'task1', 'task2')
print(r.lrange('tasks', 0, -1))  # ['task2', 'task1']

# Hashes
r.hset('user:1', mapping={'name': 'Alice', 'age': '30'})
print(r.hgetall('user:1'))  # {'name': 'Alice', 'age': '30'}

# Sets
r.sadd('tags', 'rust', 'database', 'redis')
print(r.smembers('tags'))  # {'rust', 'database', 'redis'}

# TTL
r.expire('name', 60)
print(r.ttl('name'))  # ~60
```

### redis-cli

```bash
$ redis-cli -p 6379

127.0.0.1:6379> SET mykey "Hello FerrumDB"
OK
127.0.0.1:6379> GET mykey
"Hello FerrumDB"
127.0.0.1:6379> INCR counter
(integer) 1
127.0.0.1:6379> LPUSH mylist "world" "hello"
(integer) 2
127.0.0.1:6379> LRANGE mylist 0 -1
1) "hello"
2) "world"
127.0.0.1:6379> HSET user:1 name "Bob" age "25"
(integer) 2
127.0.0.1:6379> HGETALL user:1
1) "name"
2) "Bob"
3) "age"
4) "25"
127.0.0.1:6379> INFO
# Server
ferrumdb_version:0.1.0
ferrumdb_mode:standalone
...
```

## License

Licence MIT - voir le fichier [LICENSE](LICENSE) pour plus de details

## Remerciements

- Inspiré par [Redis](https://redis.io/)
- Créé avec le runtime asynchrone [Tokio](https://tokio.rs/)
- Utilise [Axum](https://github.com/tokio-rs/axum) pour l'interface Web
- Hashage avec [siphasher](https://github.com/jedisct1/rust-siphash)

## Auteur

- Le-Stef

---

**Remarque**: FerrumDB est actuellement en cours de développement. Et avouons-le, sous Windows, en natif, c'est super pratique. Mais bien qu'il implémente les fonctionnalités principales de Redis, son utilisation n'est pas encore recommandée en production. Utilisez-le à vos propres risques.

