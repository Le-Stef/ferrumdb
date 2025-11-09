# Changelog

Toutes les modifications notables apportées à FerrumDB seront documentées dans ce fichier.

Le format est basé sur [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
et ce projet respecte [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [À venir]

### À mettre en œuvre
- Rejouer l'AOF au démarrage
- Commandes supplémentaires : LPOP, RPOP, SREM, HLEN, HEXISTS
- Commande SCAN pour une itération sécurisée des clés
- Prise en charge des ensembles triés (ZSet)
- Compactage AOF en arrière-plan
- Instantanés RDB
- Clustering multi-nœuds

## [0.3.0] - 09/11/2025

### Ajouts
- **Système de partitionnement** avec allocation automatique des partitions basée sur le CPU (16 partitions maximum)
- Hachage cohérent **SipHash13** pour la distribution des clés
- **Threads par fragment** avec mémoires isolées et fichiers AOF
- **Tableau de bord Web** sur le port 8080 avec métriques en temps réel
- **Visualisation des fragments** dans l'interface utilisateur Web affichant les statistiques par fragment
- Commande CLIENT avec sous-commandes (SETNAME, GETNAME, LIST, SETINFO, REPLY, ID)

### Modifications
- Migration d'une architecture fragmentée à thread unique vers une architecture fragmentée multithread
- Chaque fragment conserve désormais son propre fichier AOF
- Amélioration du parseur RESP2 pour gérer correctement les commandes en pipeline

### Corrections
- Bug du parseur RESP2 provoquant une consommation partielle du tampon dans les commandes en pipeline
- Le parsing des tableaux est désormais atomique (approche tout ou rien)

## [0.2.0] - 26/05/2025

### Ajouts
- **Listes** : LPUSH, RPUSH, LRANGE, LLEN
- **Ensembles** : SADD, SMEMBERS, SCARD
- **Hachages** : HSET, HGET, HGETALL, HDEL, HKEYS, HINCRBY
- **Compteurs** : INCR, INCRBY, DECR, DECRBY
- **Persistance AOF** avec sommes de contrôle xxhash64
- Commande **INFO** pour les statistiques du serveur
- Commande **KEYS** pour la recherche de clés basée sur des modèles
- **Interface utilisateur Web** pour l'administration et la surveillance
- Écriture AOF avec politiques de synchronisation configurables

### Modifications
- Extension du registre de commandes pour prendre en charge plusieurs types de données
- Amélioration de la gestion des erreurs et de la validation

## [0.1.0] - 26/03/2024

### Ajouts
- Analyseur et encodeur **du protocole RESP2**
- **Commandes de base** : SET, GET, DEL, EXISTS
- **Commandes TTL** : EXPIRE, TTL
- **Serveur TCP** utilisant le runtime asynchrone Tokio
- **Stockage en mémoire** avec HashMap et SipHasher
- Gestion **des expirations paresseuses + proactives**
- **Trait de commande** pour un système de commande extensible
- Tests unitaires et tests d'intégration
- Structure initiale du projet et documentation

### Détails techniques
- Dispatcher de commandes à thread unique
- Arc<Mutex<Dispatcher>> pour un accès thread-safe
- Architecture modulaire avec séparation claire des préoccupations