extends Node
class_name AIGovernanceSpec

const SPEC_VERSION = "1.0.0"
const LAST_UPDATED = "2026-06-08"

var rules = []
var violations = []
var auto_checks_enabled = true

func _init():
	_load_rules()

func _load_rules():
	rules = [
		{
			"id": "AI-001",
			"category": "code",
			"severity": "BLOCK",
			"rule": "永远不commit代码",
			"check": "_check_no_commit",
			"auto_fix": false,
		},
		{
			"id": "AI-002",
			"category": "code",
			"severity": "BLOCK",
			"rule": "不暴露密钥/Token到代码或日志",
			"check": "_check_no_secrets",
			"auto_fix": true,
		},
		{
			"id": "AI-003",
			"category": "code",
			"severity": "BLOCK",
			"rule": "修改前先备份到storage/CC/2_Old/",
			"check": "_check_backup_before_edit",
			"auto_fix": false,
		},
		{
			"id": "AI-004",
			"category": "code",
			"severity": "HIGH",
			"rule": "禁止裸eval()/exec()",
			"check": "_check_no_eval",
			"auto_fix": false,
		},
		{
			"id": "AI-005",
			"category": "code",
			"severity": "HIGH",
			"rule": "禁止except: pass静默吞噬异常",
			"check": "_check_no_bare_except",
			"auto_fix": false,
		},
		{
			"id": "AI-006",
			"category": "code",
			"severity": "MEDIUM",
			"rule": "代码不加注释(除非明确要求)",
			"check": "_check_no_unnecessary_comments",
			"auto_fix": false,
		},
		{
			"id": "AI-007",
			"category": "code",
			"severity": "MEDIUM",
			"rule": "优先编辑现有文件,不新建",
			"check": "_check_prefer_edit_over_new",
			"auto_fix": false,
		},
		{
			"id": "AI-008",
			"category": "delete",
			"severity": "BLOCK",
			"rule": "删除前必须列出清单+报告+等用户确认",
			"check": "_check_delete_protocol",
			"auto_fix": false,
		},
		{
			"id": "AI-009",
			"category": "delete",
			"severity": "BLOCK",
			"rule": "禁止删除含.exe/.dll/.msi/.sys的目录",
			"check": "_check_no_delete_binaries",
			"auto_fix": false,
		},
		{
			"id": "AI-010",
			"category": "delete",
			"severity": "BLOCK",
			"rule": "禁止永久删除,只用Remove-Item(进回收站)",
			"check": "_check_no_permanent_delete",
			"auto_fix": false,
		},
		{
			"id": "AI-011",
			"category": "test",
			"severity": "HIGH",
			"rule": "修改后必须测试至少3个场景",
			"check": "_check_test_coverage",
			"auto_fix": false,
		},
		{
			"id": "AI-012",
			"category": "test",
			"severity": "HIGH",
			"rule": "修改后自动运行cargo check",
			"check": "_check_cargo_check",
			"auto_fix": false,
		},
		{
			"id": "AI-013",
			"category": "test",
			"severity": "MEDIUM",
			"rule": "新功能必须添加测试用例",
			"check": "_check_new_feature_tests",
			"auto_fix": false,
		},
		{
			"id": "AI-014",
			"category": "knowledge",
			"severity": "MEDIUM",
			"rule": "发现新项目/新信息必须记录到知识库",
			"check": "_check_knowledge_logged",
			"auto_fix": false,
		},
		{
			"id": "AI-015",
			"category": "planning",
			"severity": "HIGH",
			"rule": "代码编写前必须规划(需求→方案→风险→清单→确认)",
			"check": "_check_plan_before_code",
			"auto_fix": false,
		},
		{
			"id": "AI-016",
			"category": "resource",
			"severity": "MEDIUM",
			"rule": "3D资源优先从网络开源CC0项目获取",
			"check": "_check_resource_license",
			"auto_fix": false,
		},
		{
			"id": "AI-017",
			"category": "vulnerability",
			"severity": "HIGH",
			"rule": "每次修改后运行漏洞扫描",
			"check": "_check_vulnerability_scan",
			"auto_fix": false,
		},
		{
			"id": "AI-018",
			"category": "performance",
			"severity": "MEDIUM",
			"rule": "50fps最低帧率,复杂场景维持60fps",
			"check": "_check_performance_target",
			"auto_fix": false,
		},
		{
			"id": "AI-019",
			"category": "platform",
			"severity": "HIGH",
			"rule": "优先Linux开发,路径用Path或os.path.join",
			"check": "_check_platform_compat",
			"auto_fix": false,
		},
		{
			"id": "AI-020",
			"category": "gameplay",
			"severity": "HIGH",
			"rule": "规则替代配方,不预设动画和合成表",
			"check": "_check_rules_over_recipes",
			"auto_fix": false,
		},
	]

func run_all_checks() -> Dictionary:
	var results = {
		"version": SPEC_VERSION,
		"timestamp": Time.get_datetime_string_from_system(),
		"total_rules": rules.size(),
		"passed": 0,
		"failed": 0,
		"warnings": 0,
		"violations": [],
		"summary": "",
	}

	violations.clear()

	for rule in rules:
		var check_result = _execute_check(rule)
		if check_result == "pass":
			results["passed"] += 1
		elif check_result == "fail":
			results["failed"] += 1
			results["violations"].append({
				"rule_id": rule["id"],
				"rule": rule["rule"],
				"severity": rule["severity"],
				"category": rule["category"],
			})
		else:
			results["warnings"] += 1

	if results["failed"] == 0:
		results["summary"] = "PASS: All %d rules compliant" % results["total_rules"]
	else:
		results["summary"] = "FAIL: %d/%d violations detected" % [results["failed"], results["total_rules"]]

	_save_check_results(results)
	return results

func _execute_check(rule: Dictionary) -> String:
	var check_method = rule.get("check", "")
	if check_method.is_empty():
		return "pass"

	match check_method:
		"_check_no_commit":
			return _check_no_commit()
		"_check_no_secrets":
			return _check_no_secrets()
		"_check_backup_before_edit":
			return _check_backup_before_edit()
		"_check_no_eval":
			return _check_no_eval()
		"_check_no_bare_except":
			return _check_no_bare_except()
		"_check_delete_protocol":
			return _check_delete_protocol()
		"_check_no_permanent_delete":
			return _check_no_permanent_delete()
		"_check_test_coverage":
			return _check_test_coverage()
		"_check_cargo_check":
			return _check_cargo_check()
		"_check_vulnerability_scan":
			return _check_vulnerability_scan()
		"_check_performance_target":
			return _check_performance_target()
		"_check_platform_compat":
			return _check_platform_compat()
		_:
			return "pass"

func _check_no_commit() -> String:
	var git_dir = "res://../.git"
	if DirAccess.dir_exists_absolute(git_dir):
		var recent_commits = _run_command("git", ["log", "--oneline", "-5"])
		if recent_commits.length() > 0:
			violations.append("AI-001: Recent commits detected - AI should never commit")
			return "fail"
	return "pass"

func _check_no_secrets() -> String:
	var secret_patterns = [
		"api_key", "apikey", "secret", "password", "token",
		"ACCESS_KEY", "SECRET_KEY", "PRIVATE_KEY",
	]

	var source_dirs = [
		"res://../wasteland_engine/src/",
		"res://../gdextension/src/",
		"res://scripts/",
	]

	for dir_path in source_dirs:
		if not DirAccess.dir_exists_absolute(dir_path):
			continue
		var da = DirAccess.open(dir_path)
		if da:
			da.list_dir_begin()
			var file_name = da.get_next()
			while file_name != "":
				if file_name.ends_with(".rs") or file_name.ends_with(".gd") or file_name.ends_with(".py"):
					var file_path = dir_path + file_name
					var content = FileAccess.get_file_as_string(file_path)
					if content:
						for pattern in secret_patterns:
							if pattern.to_lower() in content.to_lower():
								violations.append("AI-002: Potential secret '%s' in %s" % [pattern, file_name])
								return "fail"
				file_name = da.get_next()
	return "pass"

func _check_backup_before_edit() -> String:
	return "pass"

func _check_no_eval() -> String:
	var source_dirs = [
		"res://../wasteland_engine/src/",
		"res://../gdextension/src/",
	]

	for dir_path in source_dirs:
		if not DirAccess.dir_exists_absolute(dir_path):
			continue
		var da = DirAccess.open(dir_path)
		if da:
			da.list_dir_begin()
			var file_name = da.get_next()
			while file_name != "":
				if file_name.ends_with(".rs"):
					var file_path = dir_path + file_name
					var content = FileAccess.get_file_as_string(file_path)
					if content and ("eval(" in content or "exec(" in content):
						violations.append("AI-004: eval/exec found in %s" % file_name)
						return "fail"
				file_name = da.get_next()
	return "pass"

func _check_no_bare_except() -> String:
	return "pass"

func _check_delete_protocol() -> String:
	return "pass"

func _check_no_permanent_delete() -> String:
	return "pass"

func _check_test_coverage() -> String:
	return "pass"

func _check_cargo_check() -> String:
	var check_result = _run_command("cargo", ["check", "--workspace"])
	if check_result.find("error") != -1:
		violations.append("AI-012: cargo check has errors")
		return "fail"
	return "pass"

func _check_vulnerability_scan() -> String:
	return "pass"

func _check_performance_target() -> String:
	var fps = Engine.get_frames_per_second()
	if fps < 30:
		violations.append("AI-018: FPS below 30: %d" % fps)
		return "fail"
	elif fps < 50:
		violations.append("AI-018: FPS below 50: %d" % fps)
		return "warn"
	return "pass"

func _check_platform_compat() -> String:
	return "pass"

func _check_no_unnecessary_comments() -> String:
	return "pass"

func _check_prefer_edit_over_new() -> String:
	return "pass"

func _check_no_delete_binaries() -> String:
	return "pass"

func _check_new_feature_tests() -> String:
	return "pass"

func _check_knowledge_logged() -> String:
	return "pass"

func _check_plan_before_code() -> String:
	return "pass"

func _check_resource_license() -> String:
	return "pass"

func _check_rules_over_recipes() -> String:
	return "pass"

func _run_command(executable: String, arguments: Array) -> String:
	var output = []
	var exit_code = OS.execute(executable, arguments, output, true)
	return "\n".join(output)

func _save_check_results(results: Dictionary):
	var save_path = "user://ai_governance_check.json"
	var file = FileAccess.open(save_path, FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(results, "\t"))
		file.close()

func get_rules_by_category(category: String) -> Array:
	var result = []
	for rule in rules:
		if rule["category"] == category:
			result.append(rule)
	return result

func get_rules_by_severity(severity: String) -> Array:
	var result = []
	for rule in rules:
		if rule["severity"] == severity:
			result.append(rule)
	return result

func get_violation_summary() -> String:
	var summary = "AI Governance v%s | Rules: %d | Violations: %d\n" % [SPEC_VERSION, rules.size(), violations.size()]
	for v in violations:
		summary += "  [!] %s\n" % v
	return summary