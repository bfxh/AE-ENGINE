extends Node
class_name InventorySystem

var _items: Dictionary = {}
var _max_slots: int = 40
var _max_weight: float = 50.0
var _current_weight: float = 0.0

signal item_added(item_id: String, count: int)
signal item_removed(item_id: String, count: int)
signal inventory_full(item_id: String)
signal weight_exceeded(item_id: String)
signal inventory_changed()

const ITEM_DATABASE = {
	"scrap_metal": {"name": "Scrap Metal", "weight": 0.5, "stack_size": 50, "category": "material", "value": 5},
	"cloth": {"name": "Cloth", "weight": 0.1, "stack_size": 30, "category": "material", "value": 3},
	"wood": {"name": "Wood", "weight": 0.3, "stack_size": 40, "category": "material", "value": 2},
	"stone": {"name": "Stone", "weight": 0.8, "stack_size": 30, "category": "material", "value": 1},
	"iron_ore": {"name": "Iron Ore", "weight": 1.0, "stack_size": 20, "category": "material", "value": 8},
	"copper_wire": {"name": "Copper Wire", "weight": 0.2, "stack_size": 50, "category": "material", "value": 10},
	"fiber": {"name": "Plant Fiber", "weight": 0.05, "stack_size": 60, "category": "material", "value": 1},
	"herb": {"name": "Medicinal Herb", "weight": 0.05, "stack_size": 20, "category": "consumable", "value": 5},
	"water_bottle": {"name": "Water Bottle", "weight": 0.5, "stack_size": 10, "category": "consumable", "value": 8},
	"medkit": {"name": "Medkit", "weight": 0.3, "stack_size": 5, "category": "consumable", "value": 25},
	"ammo_9mm": {"name": "9mm Ammo", "weight": 0.01, "stack_size": 100, "category": "ammo", "value": 2},
	"ammo_shotgun": {"name": "Shotgun Shells", "weight": 0.03, "stack_size": 50, "category": "ammo", "value": 3},
	"weapon_parts": {"name": "Weapon Parts", "weight": 0.4, "stack_size": 20, "category": "component", "value": 15},
	"electronic_parts": {"name": "Electronic Parts", "weight": 0.1, "stack_size": 30, "category": "component", "value": 12},
	"fuel": {"name": "Fuel", "weight": 0.8, "stack_size": 10, "category": "consumable", "value": 10},
	"food_can": {"name": "Canned Food", "weight": 0.4, "stack_size": 15, "category": "consumable", "value": 6},
	"knife": {"name": "Combat Knife", "weight": 0.3, "stack_size": 1, "category": "weapon", "value": 20},
	"pistol": {"name": "Pistol", "weight": 0.9, "stack_size": 1, "category": "weapon", "value": 50},
	"rifle": {"name": "Rifle", "weight": 3.5, "stack_size": 1, "category": "weapon", "value": 80},
	"armor_scrap": {"name": "Scrap Armor", "weight": 5.0, "stack_size": 1, "category": "armor", "value": 30},
	"backpack": {"name": "Backpack", "weight": 1.0, "stack_size": 1, "category": "equipment", "value": 40},
	"flashlight": {"name": "Flashlight", "weight": 0.3, "stack_size": 1, "category": "tool", "value": 15},
	"rope": {"name": "Rope", "weight": 1.5, "stack_size": 5, "category": "tool", "value": 8},
	"lockpick": {"name": "Lockpick Set", "weight": 0.1, "stack_size": 1, "category": "tool", "value": 25},
	"geiger_counter": {"name": "Geiger Counter", "weight": 0.4, "stack_size": 1, "category": "tool", "value": 35},
	"sap": {"name": "Tree Sap", "weight": 0.1, "stack_size": 20, "category": "material", "value": 4},
}

func add_item(item_id: String, count: int = 1) -> bool:
	if not ITEM_DATABASE.has(item_id):
		return false

	var item_data = ITEM_DATABASE[item_id]
	var added_weight = item_data["weight"] * count

	if _current_weight + added_weight > _max_weight:
		weight_exceeded.emit(item_id)
		return false

	if _items.has(item_id):
		var current = _items[item_id]["count"]
		var max_stack = item_data["stack_size"]
		if current + count > max_stack:
			_items[item_id]["count"] = max_stack
			item_added.emit(item_id, max_stack - current)
		else:
			_items[item_id]["count"] = current + count
			item_added.emit(item_id, count)
	else:
		if get_used_slots() >= _max_slots:
			inventory_full.emit(item_id)
			return false
		_items[item_id] = {
			"count": count,
			"data": item_data,
		}
		item_added.emit(item_id, count)

	_current_weight += added_weight
	inventory_changed.emit()
	return true

func remove_item(item_id: String, count: int = 1) -> bool:
	if not _items.has(item_id):
		return false

	var current = _items[item_id]["count"]
	if current < count:
		return false

	_items[item_id]["count"] = current - count
	_current_weight -= ITEM_DATABASE[item_id]["weight"] * count

	if _items[item_id]["count"] <= 0:
		_items.erase(item_id)

	item_removed.emit(item_id, count)
	inventory_changed.emit()
	return true

func has_item(item_id: String, count: int = 1) -> bool:
	if not _items.has(item_id):
		return false
	return _items[item_id]["count"] >= count

func get_item_count(item_id: String) -> int:
	if not _items.has(item_id):
		return 0
	return _items[item_id]["count"]

func get_all_items() -> Dictionary:
	return _items.duplicate(true)

func get_used_slots() -> int:
	return _items.size()

func get_current_weight() -> float:
	return _current_weight

func get_max_weight() -> float:
	return _max_weight

func get_items_by_category(category: String) -> Array:
	var result = []
	for item_id in _items:
		if _items[item_id]["data"]["category"] == category:
			result.append({"id": item_id, "count": _items[item_id]["count"], "data": _items[item_id]["data"]})
	return result

func get_total_value() -> int:
	var total = 0
	for item_id in _items:
		total += _items[item_id]["data"]["value"] * _items[item_id]["count"]
	return total

func transfer_to(other: InventorySystem, item_id: String, count: int = 1) -> bool:
	if not remove_item(item_id, count):
		return false
	if not other.add_item(item_id, count):
		add_item(item_id, count)
		return false
	return true

func use_item(item_id: String) -> Dictionary:
	if not has_item(item_id):
		return {"success": false, "reason": "not_found"}

	var item_data = ITEM_DATABASE[item_id]
	var result = {"success": false, "item_id": item_id}

	match item_data["category"]:
		"consumable":
			result = _use_consumable(item_id)
		"weapon":
			result = {"success": true, "action": "equip", "item_id": item_id}
		"armor":
			result = {"success": true, "action": "equip", "item_id": item_id}
		"tool":
			result = {"success": true, "action": "equip", "item_id": item_id}
		"equipment":
			result = {"success": true, "action": "equip", "item_id": item_id}
		_:
			result = {"success": false, "reason": "not_usable"}

	return result

func _use_consumable(item_id: String) -> Dictionary:
	var effects = {
		"water_bottle": {"thirst": -30, "health": 0},
		"medkit": {"thirst": 0, "health": 50},
		"food_can": {"thirst": 0, "health": 10, "hunger": -25},
		"herb": {"thirst": 0, "health": 15, "radiation": -5},
		"fuel": {"thirst": 0, "health": -20},
	}

	if effects.has(item_id):
		remove_item(item_id, 1)
		return {"success": true, "action": "consume", "item_id": item_id, "effects": effects[item_id]}

	return {"success": false, "reason": "no_effect"}

func clear():
	_items.clear()
	_current_weight = 0.0
	inventory_changed.emit()

func get_debug_info() -> Dictionary:
	return {
		"slots_used": get_used_slots(),
		"slots_max": _max_slots,
		"weight": _current_weight,
		"weight_max": _max_weight,
		"items": _items.keys().size(),
		"total_value": get_total_value(),
	}
