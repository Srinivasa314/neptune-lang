#include "util.h"
#include <cstdint>
#include <vector>
#pragma once

namespace neptune_vm {

template <typename Entry, typename Hash, typename Equal, typename Empty,
          typename Allocator, uint32_t default_size>
class HashTableIterator;

template <typename Entry, typename Hash, typename Equal, typename Empty,
          typename Allocator, uint32_t default_size>
class HashTableConstIterator;

template <typename Entry, typename Hash, typename Equal, typename Empty,
          typename Allocator, uint32_t default_size = 4>
class HashTable {
public:
  using iterator =
      HashTableIterator<Entry, Hash, Equal, Empty, Allocator, default_size>;
  using const_iterator = HashTableConstIterator<Entry, Hash, Equal, Empty,
                                                Allocator, default_size>;

  HashTable() : HashTable(default_size) {}
  HashTable(uint32_t size) : alloc(Allocator{}), size_(0) {
    capacity = power_of_two_ceil(2 * size);
    if (capacity < default_size)
      capacity = power_of_two_ceil(2 * default_size);
    entries = alloc.allocate(capacity);
    for (uint32_t i = 0; i < capacity; i++)
      alloc.construct(&entries[i], Empty{}.empty());
  }

  HashTable(HashTable &other) : HashTable() {
    for (uint32_t i = 0; i < other.capacity; i++)
      if (!(Empty{}.is_empty(other.entries[i])))
        insert(other.entries[i]);
  }

  friend void swap(HashTable &first, HashTable &second) {
    using std::swap;
    swap(first.alloc, second.alloc);
    swap(first.entries, second.entries);
    swap(first.size_, second.size_);
    swap(first.capacity, second.capacity);
  }

  HashTable(HashTable &&other) noexcept {
    entries = nullptr;
    swap(*this, other);
  }

  HashTable &operator=(HashTable &other) {
    HashTable tmp(other);
    swap(*this, tmp);
    return *this;
  }

  HashTable &operator=(HashTable &&other) {
    swap(*this, other);
    return *this;
  }

  template <typename Key> const_iterator find(Key key) const {
    return const_cast<HashTable *>(this)->find(key);
  }

  template <typename Key>
  ALWAYS_INLINE iterator find(Key key) {
    uint32_t index = Hash{}(key) & (capacity - 1);
    for (uint32_t i = index;; i = (i + 1) & (capacity - 1)) {
      if (likely(Equal{}(entries[i], key))) {
        return iterator(this, &entries[i]);
      }
      if (Empty{}.is_empty(entries[i])) {
        return iterator(this, nullptr);
      }
    }
  }
  ALWAYS_INLINE bool insert(Entry e) {
    if (unlikely((size_ + 1) * 2 > capacity)) {
      reserve(size_ + 1);
    }
    uint32_t index = Hash{}(e) & (capacity - 1);
    for (uint32_t i = index;; i = (i + 1) & (capacity - 1)) {
      if (likely(Equal{}(entries[i], e))) {
        entries[i] = e;
        return false;
      }
      if (Empty{}.is_empty(entries[i])) {
        entries[i] = e;
        size_++;
        return true;
      }
    }
  }
  iterator begin() { return iterator(this, next_entry(entries)); }
  iterator end() { return iterator(this, nullptr); }

  const_iterator begin() const {
    return const_iterator(this, next_entry(entries));
  }
  const_iterator end() const { return const_iterator(this, nullptr); }

  template <typename Key> bool erase(Key k) {
    iterator it = find(k);
    if (it == end())
      return false;
    else {
      erase(it);
      return true;
    }
  }

  void erase(iterator it) {
    uint32_t bucket = it.inner - entries;
    for (uint32_t i = (bucket + 1) & (capacity - 1);;
         i = (i + 1) & (capacity - 1)) {
      if (Empty{}.is_empty(entries[i])) {
        entries[bucket] = Empty{}.empty();
        size_--;
        return;
      }
      uint32_t ideal = Hash{}(entries[i]) & (capacity - 1);
      if (diff(bucket, ideal) < diff(i, ideal)) {
        entries[bucket] = entries[i];
        bucket = i;
      }
    }
  }
  void clear() { *this = HashTable(); }
  uint32_t size() const { return size_; }
  template <typename Key> bool count(Key k) const { return find(k) != end(); }

  ~HashTable() {
    if (entries) {
      for (uint32_t i = 0; i < capacity; i++) {
        alloc.destroy(&entries[i]);
      }
      alloc.deallocate(entries, capacity);
    }
  }

private:
  Allocator alloc;
  Entry *entries;
  uint32_t size_;
  uint32_t capacity;

  void reserve(uint32_t size) {
    if (size * 2 > capacity) {
      HashTable tmp(size);
      for (uint32_t i = 0; i < capacity; i++)
        if (!(Empty{}.is_empty(entries[i])))
          tmp.insert(entries[i]);
      *this = std::move(tmp);
    }
  }
  uint32_t diff(uint32_t a, uint32_t b) const {
    return (capacity + (a - b)) & (capacity - 1);
  }
  Entry *next_entry(Entry *entry) const {
    auto end = entries + capacity;
    Entry *e;
    for (e = entry; e < end && (Empty{}.is_empty(*e)); e++)
      ;
    if (e == end)
      return nullptr;
    else
      return e;
  }
  friend class HashTableIterator<Entry, Hash, Equal, Empty, Allocator,
                                 default_size>;
  friend class HashTableConstIterator<Entry, Hash, Equal, Empty, Allocator,
                                      default_size>;
};

template <typename Entry, typename Hash, typename Equal, typename Empty,
          typename Allocator, uint32_t default_size>
class HashTableIterator {
private:
  using hash_table_t =
      HashTable<Entry, Hash, Equal, Empty, Allocator, default_size>;

  HashTableIterator(hash_table_t *hash_table, Entry *e)
      : inner(e), hash_table(hash_table) {}
  Entry *inner;
  hash_table_t *hash_table;
  friend class HashTable<Entry, Hash, Equal, Empty, Allocator, default_size>;
  friend class HashTableConstIterator<Entry, Hash, Equal, Empty, Allocator,
                                      default_size>;

public:
  HashTableIterator operator++() {
    inner = hash_table->next_entry(inner + 1);
    return *this;
  }
  Entry &operator*() { return *inner; }
  Entry *operator->() { return inner; }
  bool operator==(HashTableIterator other) { return inner == other.inner; }
  bool operator!=(HashTableIterator other) { return inner != other.inner; }
};

template <typename Entry, typename Hash, typename Equal, typename Empty,
          typename Allocator, uint32_t default_size>
class HashTableConstIterator {
private:
  using hash_table_t =
      const HashTable<Entry, Hash, Equal, Empty, Allocator, default_size>;
  HashTableConstIterator(
      HashTableIterator<Entry, Hash, Equal, Empty, Allocator, default_size> it)
      : inner(it.inner), hash_table(it.hash_table) {}
  HashTableConstIterator(hash_table_t *hash_table, const Entry *e)
      : inner(e), hash_table(hash_table) {}
  const Entry *inner;
  hash_table_t *hash_table;
  friend class HashTable<Entry, Hash, Equal, Empty, Allocator, default_size>;

public:
  HashTableConstIterator operator++() {
    inner = hash_table->next_entry(inner + 1);
    return *this;
  }
  const Entry &operator*() { return *inner; }
  const Entry *operator->() { return inner; }
  bool operator==(HashTableConstIterator other) { return inner == other.inner; }
  bool operator!=(HashTableConstIterator other) { return inner != other.inner; }
};

template <typename T, typename Hash, typename Equal, typename Empty,
          typename Allocator = std::allocator<T>>
using HashSet = HashTable<T, Hash, Equal, Empty, Allocator>;

template <typename Hash, typename K, typename V> class HashMapHash {
public:
  uint32_t operator()(std::pair<K, V> entry) { return Hash{}(entry.first); }
  template <typename K2> uint32_t operator()(K2 key2) { return Hash{}(key2); }
};

template <typename Equal, typename K, typename V> class HashMapEqual {
public:
  bool operator()(std::pair<K, V> p1, std::pair<K, V> p2) {
    return Equal{}(p1.first, p2.first);
  }
  template <typename K2> bool operator()(std::pair<K, V> entry, K2 key2) {
    return Equal{}(entry.first, key2);
  }
  template <typename K2> bool operator()(K2 key2, std::pair<K, V> entry) {
    return Equal{}(key2, entry.first);
  }
};

template <typename Empty, typename K, typename V> class HashMapEmpty {
public:
  bool is_empty(std::pair<K, V> pair) { return Empty{}.is_empty(pair.first); }
  std::pair<K, V> empty() { return std::pair<K, V>(Empty{}.empty(), V{}); }
};

template <typename K, typename V, typename Hash, typename Equal, typename Empty,
          typename Allocator = std::allocator<std::pair<K, V>>>
using HashMap =
    HashTable<std::pair<K, V>, HashMapHash<Hash, K, V>,
              HashMapEqual<Equal, K, V>, HashMapEmpty<Empty, K, V>, Allocator>;

template <typename T> class NullptrEmpty {
public:
  bool is_empty(T *t) { return t == nullptr; }
  T *empty() { return nullptr; }
};
} // namespace neptune_vm