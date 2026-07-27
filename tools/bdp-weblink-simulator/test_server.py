"""Tests exhaustivos del simulador BDP WebLink.

Cubre: endpoints, validación de datos, fault injection, idempotencia,
pagos parciales, sobrepago, cancelación, facturación, clientes, y edge cases.
"""

import json
import threading
import unittest
import urllib.error
import urllib.request
from pathlib import Path

from server import build_server


ADMIN_KEY = "clave-local-pruebas-seguras"


class SimulatorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = build_server("127.0.0.1", 0, Path(__file__).with_name("fixtures.json"), ADMIN_KEY)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.base = f"http://127.0.0.1:{cls.server.server_port}"

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self):
        self.post("/__simulator/reset", {}, admin=True)
        response = self.post("/Auth/Login", {"Login": "local", "Password": "secret", "CodigoIntegrador": "SIM"})
        self.token = response["AuthSession"]["Token"]

    def request(self, method, path, payload=None, *, auth=False, admin=False, timeout=5):
        data = None if payload is None else json.dumps(payload).encode()
        request = urllib.request.Request(self.base + path, data=data, method=method, headers={"Content-Type": "application/json"})
        if auth:
            request.add_header("Authorization", f"Bearer {self.token}")
        if admin:
            request.add_header("X-Simulator-Key", ADMIN_KEY)
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return json.loads(response.read())

    def post(self, path, payload, **kwargs):
        return self.request("POST", path, payload, **kwargs)

    @staticmethod
    def order_payload(operation=0, marketplace_id="GLOCAL00000001", total=10, items=None):
        if items is None:
            items = [{"Id": 1000000000001, "Name": "Articulo", "Units": 1, "Price": total, "Total": total}]
        return {"EmployeeId": 1, "ItemsProfileId": 1, "OrderEndType": 0, "OrderOperationType": operation,
                "Order": {"MarketId": 77, "MarketplaceOrderId": marketplace_id, "PosId": 1, "Total": total,
                          "Items": items}}

    def create_order(self, **kwargs):
        return self.post("/API/Orders/Create", self.order_payload(**kwargs), auth=True)

    def create_and_pay_order(self, total=10, marketplace_id="GLOCAL00000001", tender_id=1, payment_id="PAY-LOCAL-1"):
        """Helper: crea orden, paga completo, retorna order_id."""
        order = self.create_order(total=total, marketplace_id=marketplace_id)
        order_id = order["OrderId"]
        identifier = {"OrderId": order_id}
        payment = {"OrderIdentifier": identifier, "Payment": {"TenderId": tender_id, "Amount": total, "PaymentId": payment_id}}
        self.post("/API/Orders/Payment/Add", payment, auth=True)
        return order_id

    # ═══════════════════════════════════════════════════════════════
    # 1. SEGURIDAD Y AUTENTICACIÓN
    # ═══════════════════════════════════════════════════════════════

    def test_rejects_non_loopback_bind(self):
        with self.assertRaises(ValueError):
            build_server("0.0.0.0", 0, Path(__file__).with_name("fixtures.json"), ADMIN_KEY)

    def test_auth_is_required(self):
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/API/Orders/Get", {})
        self.assertEqual(ctx.exception.code, 401)

    def test_rejects_invalid_token(self):
        """Token que no fue generado por login debe ser rechazado."""
        request = urllib.request.Request(self.base + "/API/Orders/Get", data=json.dumps({}).encode(), method="POST",
                                         headers={"Content-Type": "application/json", "Authorization": "Bearer token-falso"})
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            urllib.request.urlopen(request, timeout=5)
        self.assertEqual(ctx.exception.code, 401)

    def test_rejects_malformed_auth_header(self):
        """Authorization sin 'Bearer ' debe ser rechazado."""
        request = urllib.request.Request(self.base + "/API/Orders/Get", data=json.dumps({}).encode(), method="POST",
                                         headers={"Content-Type": "application/json", "Authorization": self.token})
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            urllib.request.urlopen(request, timeout=5)
        self.assertEqual(ctx.exception.code, 401)

    def test_admin_requires_valid_key(self):
        """Admin endpoint rechaza clave incorrecta."""
        request = urllib.request.Request(self.base + "/__simulator/reset", data=json.dumps({}).encode(), method="POST",
                                         headers={"Content-Type": "application/json", "X-Simulator-Key": "clave-incorrecta"})
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            urllib.request.urlopen(request, timeout=5)
        self.assertEqual(ctx.exception.code, 403)

    def test_admin_requires_key(self):
        """Admin endpoint rechaza sin clave."""
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/__simulator/reset", {})
        self.assertEqual(ctx.exception.code, 403)

    # ═══════════════════════════════════════════════════════════════
    # 2. ENDPOINTS PÚBLICOS (HEALTH, LOGIN, VERSION)
    # ═══════════════════════════════════════════════════════════════

    def test_health_returns_is_alive(self):
        """Health check no requiere auth y devuelve IsAlive."""
        response = self.post("/Service/Health", {})
        self.assertTrue(response["IsAlive"])

    def test_login_returns_valid_session(self):
        """Login devuelve AuthSession con Token y ExpiresIn."""
        response = self.post("/Auth/Login", {"Login": "local", "Password": "secret", "CodigoIntegrador": "SIM"})
        self.assertIn("AuthSession", response)
        session = response["AuthSession"]
        self.assertTrue(session["Token"])
        self.assertGreater(session["ExpiresIn_InSecconds"], 0)
        self.assertEqual(response["ErrorMessage"], "")

    def test_login_rejects_missing_credentials(self):
        """Login sin credenciales completas falla."""
        response = self.post("/Auth/Login", {"Login": "", "Password": "secret", "CodigoIntegrador": "SIM"})
        self.assertIn("credenciales incompletas", response["ErrorMessage"])
        self.assertIsNone(response["AuthSession"])

    def test_login_rejects_partial_credentials(self):
        """Login sin CodigoIntegrador falla."""
        response = self.post("/Auth/Login", {"Login": "local", "Password": "secret"})
        self.assertIn("credenciales incompletas", response["ErrorMessage"])

    def test_get_version_returns_simulator_info(self):
        """GetVersion devuelve info del simulador."""
        response = self.post("/Service/GetVersion", {}, auth=True)
        self.assertIn("SIMULATOR", response.get("Revision", ""))
        self.assertEqual(response["ErrorMessage"], "")

    # ═══════════════════════════════════════════════════════════════
    # 3. CATÁLOGO (ARTICLES, TENDERS, CUSTOMERS, DEPARTMENTS, ROOMS)
    # ═══════════════════════════════════════════════════════════════

    def test_export_articles_returns_array(self):
        """Export articles devuelve lista de artículos."""
        response = self.post("/API/Articles/Export", {}, auth=True)
        self.assertIn("Articles", response)
        self.assertIsInstance(response["Articles"], list)
        self.assertGreater(len(response["Articles"]), 0)
        self.assertEqual(response["ErrorMessage"], "")

    def test_get_pos_list_returns_articles(self):
        """GetPOSList también devuelve artículos."""
        response = self.post("/API/Articles/GetPOSList", {}, auth=True)
        self.assertIn("Articles", response)
        self.assertIsInstance(response["Articles"], list)

    def test_export_customers_returns_array(self):
        """Export customers devuelve lista de clientes."""
        response = self.post("/API/Customers/Export", {}, auth=True)
        self.assertIn("Customers", response)
        self.assertIsInstance(response["Customers"], list)
        self.assertGreater(len(response["Customers"]), 0)

    def test_export_departments_returns_array(self):
        """Export departments devuelve departamentos."""
        response = self.post("/API/Departments/Export", {}, auth=True)
        self.assertIn("Departments", response)
        self.assertIsInstance(response["Departments"], list)

    def test_export_departments_from_profile(self):
        """Departments/ExportFromProfile también funciona."""
        response = self.post("/API/Departments/ExportFromProfile", {}, auth=True)
        self.assertIn("Departments", response)

    def test_get_tenders_returns_array(self):
        """Get tenders devuelve métodos de pago."""
        response = self.post("/API/Tenders/GetList", {}, auth=True)
        self.assertIn("Tenders", response)
        self.assertIsInstance(response["Tenders"], list)

    def test_get_pos_tenders(self):
        """GetPOSList tenders también funciona."""
        response = self.post("/API/Tenders/GetPOSList", {}, auth=True)
        self.assertIn("Tenders", response)

    def test_get_rooms_tables(self):
        """GetTables devuelve salones y mesas."""
        response = self.post("/API/Rooms/GetTables", {}, auth=True)
        self.assertIn("Rooms", response)
        self.assertIn("Tables", response)

    def test_get_room_tables_alias(self):
        """Room/GetTables es alias de Rooms/GetTables."""
        response = self.post("/API/Room/GetTables", {}, auth=True)
        self.assertIn("Rooms", response)

    def test_get_employees(self):
        """Employees/Get devuelve empleados."""
        response = self.post("/API/Employees/Get", {}, auth=True)
        self.assertIn("Employees", response)

    def test_get_pos_employees(self):
        """POS/Employees/Get también funciona."""
        response = self.post("/API/POS/Employees/Get", {}, auth=True)
        self.assertIn("Employees", response)

    def test_get_poses(self):
        """POSes/Get devuelve POSes."""
        response = self.post("/API/POSes/Get", {}, auth=True)
        self.assertIn("POSes", response)

    def test_get_pos_single(self):
        """POS/Get es alias."""
        response = self.post("/API/POS/Get", {}, auth=True)
        self.assertIn("POSes", response)

    # ═══════════════════════════════════════════════════════════════
    # 4. CREAR ORDEN — CASOS BÁSICOS
    # ═══════════════════════════════════════════════════════════════

    def test_check_order_does_not_persist(self):
        response = self.create_order(operation=1)
        self.assertTrue(response["Checked"])
        state = self.request("GET", "/__simulator/state", admin=True)
        self.assertEqual(state["Orders"], [])

    def test_create_order_is_idempotent_and_detects_conflict(self):
        first = self.create_order()
        second = self.create_order()
        self.assertEqual(first["OrderId"], second["OrderId"])
        self.assertTrue(second["Duplicate"])
        conflict = self.create_order(total=11)
        self.assertIn("payload diferente", conflict["ErrorMessage"])

    def test_create_order_assigns_incremental_ids(self):
        """Cada orden nueva tiene un OrderId único e incremental."""
        first = self.create_order(marketplace_id="GORDER00000001")
        second = self.create_order(marketplace_id="GORDER00000002")
        self.assertEqual(second["OrderId"], first["OrderId"] + 1)

    def test_create_order_returns_order_object(self):
        """La respuesta incluye el objeto Order completo."""
        response = self.create_order()
        self.assertIn("Order", response)
        order = response["Order"]
        self.assertEqual(order["MarketplaceOrderId"], "GLOCAL00000001")
        self.assertEqual(order["Status"], 0)
        self.assertEqual(order["Payments"], [])

    def test_create_order_rejects_missing_order(self):
        """Sin campo Order → error."""
        payload = {"EmployeeId": 1, "ItemsProfileId": 1, "OrderEndType": 0, "OrderOperationType": 0}
        response = self.post("/API/Orders/Create", payload, auth=True)
        self.assertIn("Order obligatorio", response["ErrorMessage"])

    def test_create_order_rejects_empty_items(self):
        """Items vacío → error."""
        payload = self.order_payload(items=[])
        response = self.post("/API/Orders/Create", payload, auth=True)
        self.assertIn("obligatorios", response["ErrorMessage"])

    def test_create_order_rejects_negative_price(self):
        """Price negativo → error."""
        items = [{"Id": 1, "Name": "X", "Units": 1, "Price": -5, "Total": -5}]
        payload = self.order_payload(items=items)
        response = self.post("/API/Orders/Create", payload, auth=True)
        self.assertIn("invalida", response["ErrorMessage"])

    def test_create_order_rejects_zero_units(self):
        """Units = 0 → error."""
        items = [{"Id": 1, "Name": "X", "Units": 0, "Price": 5, "Total": 0}]
        payload = self.order_payload(items=items)
        response = self.post("/API/Orders/Create", payload, auth=True)
        self.assertIn("invalida", response["ErrorMessage"])

    def test_create_order_rejects_missing_market_id(self):
        """Sin MarketId → error."""
        payload = {"EmployeeId": 1, "ItemsProfileId": 1, "OrderEndType": 0, "OrderOperationType": 0,
                   "Order": {"MarketplaceOrderId": "GTEST001", "PosId": 1, "Items": [{"Id": 1, "Name": "X", "Units": 1, "Price": 5}]}}
        response = self.post("/API/Orders/Create", payload, auth=True)
        self.assertIn("obligatorios", response["ErrorMessage"])

    def test_create_order_rejects_missing_marketplace_order_id(self):
        """Sin MarketplaceOrderId → error."""
        payload = {"EmployeeId": 1, "ItemsProfileId": 1, "OrderEndType": 0, "OrderOperationType": 0,
                   "Order": {"MarketId": 77, "PosId": 1, "Items": [{"Id": 1, "Name": "X", "Units": 1, "Price": 5}]}}
        response = self.post("/API/Orders/Create", payload, auth=True)
        self.assertIn("obligatorios", response["ErrorMessage"])

    def test_create_order_multiple_items(self):
        """Orden con múltiples artículos."""
        items = [
            {"Id": 1, "Name": "Café", "Units": 2, "Price": 2.50, "Total": 5.00},
            {"Id": 2, "Name": "Tostada", "Units": 1, "Price": 3.50, "Total": 3.50},
        ]
        response = self.create_order(total=8.50, items=items)
        self.assertIn("OrderId", response)
        self.assertGreater(response["OrderId"], 0)

    # ═══════════════════════════════════════════════════════════════
    # 5. PAGOS — CASOS COMPLETOS
    # ═══════════════════════════════════════════════════════════════

    def test_payment_is_idempotent_and_invoice_requires_full_payment(self):
        order_id = self.create_order()["OrderId"]
        identifier = {"OrderId": order_id}
        early = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertIn("saldo pendiente", early["ErrorMessage"])
        payment = {"OrderIdentifier": identifier, "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-LOCAL-1"}}
        self.assertEqual(self.post("/API/Orders/Payment/Add", payment, auth=True)["Balance"], 0)
        self.assertTrue(self.post("/API/Orders/Payment/Add", payment, auth=True)["Duplicate"])
        invoice = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertRegex(invoice["InvoiceNumber"], r"^SIM-\d{6}$")

    def test_partial_payment_returns_remaining_balance(self):
        """Pago parcial devuelve Balance > 0."""
        order_id = self.create_order(total=20)["OrderId"]
        identifier = {"OrderId": order_id}
        payment = {"OrderIdentifier": identifier, "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-PARTIAL-1"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertEqual(response["Balance"], 10.0)
        self.assertEqual(response["ErrorMessage"], "")

    def test_second_partial_payment_completes(self):
        """Dos pagos parciales suman el total y permiten facturar."""
        order_id = self.create_order(total=20)["OrderId"]
        identifier = {"OrderId": order_id}
        self.post("/API/Orders/Payment/Add", {"OrderIdentifier": identifier,
            "Payment": {"TenderId": 1, "Amount": 12, "PaymentId": "PAY-P1"}}, auth=True)
        response = self.post("/API/Orders/Payment/Add", {"OrderIdentifier": identifier,
            "Payment": {"TenderId": 1, "Amount": 8, "PaymentId": "PAY-P2"}}, auth=True)
        self.assertEqual(response["Balance"], 0.0)
        invoice = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertIn("InvoiceNumber", invoice)

    def test_overpayment_is_rejected(self):
        """Pago mayor al saldo pendiente es rechazado."""
        order_id = self.create_order(total=10)["OrderId"]
        identifier = {"OrderId": order_id}
        payment = {"OrderIdentifier": identifier, "Payment": {"TenderId": 1, "Amount": 15, "PaymentId": "PAY-OVER-1"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("saldo pendiente", response["ErrorMessage"])

    def test_payment_to_nonexistent_order_fails(self):
        """Pago a orden inexistente falla."""
        payment = {"OrderIdentifier": {"OrderId": 99999}, "Payment": {"TenderId": 1, "Amount": 5, "PaymentId": "PAY-NOPE"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("inexistente", response["ErrorMessage"])

    def test_payment_with_zero_amount_fails(self):
        """Pago con Amount=0 falla."""
        order_id = self.create_order()["OrderId"]
        payment = {"OrderIdentifier": {"OrderId": order_id}, "Payment": {"TenderId": 1, "Amount": 0, "PaymentId": "PAY-ZERO"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("obligatorios", response["ErrorMessage"])

    def test_payment_with_negative_amount_fails(self):
        """Pago con Amount negativo falla."""
        order_id = self.create_order()["OrderId"]
        payment = {"OrderIdentifier": {"OrderId": order_id}, "Payment": {"TenderId": 1, "Amount": -5, "PaymentId": "PAY-NEG"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("obligatorios", response["ErrorMessage"])

    def test_payment_with_zero_tender_fails(self):
        """Pago con TenderId=0 falla."""
        order_id = self.create_order()["OrderId"]
        payment = {"OrderIdentifier": {"OrderId": order_id}, "Payment": {"TenderId": 0, "Amount": 10, "PaymentId": "PAY-NOTENDER"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("obligatorios", response["ErrorMessage"])

    def test_payment_without_payment_id_fails(self):
        """Pago sin PaymentId falla."""
        order_id = self.create_order()["OrderId"]
        payment = {"OrderIdentifier": {"OrderId": order_id}, "Payment": {"TenderId": 1, "Amount": 10}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("obligatorios", response["ErrorMessage"])

    def test_duplicate_payment_id_different_amount_rejected(self):
        """Mismo PaymentId con diferente Amount es rechazado."""
        order_id = self.create_order(total=20)["OrderId"]
        identifier = {"OrderId": order_id}
        self.post("/API/Orders/Payment/Add", {"OrderIdentifier": identifier,
            "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-DUP-ID"}}, auth=True)
        response = self.post("/API/Orders/Payment/Add", {"OrderIdentifier": identifier,
            "Payment": {"TenderId": 1, "Amount": 5, "PaymentId": "PAY-DUP-ID"}}, auth=True)
        self.assertIn("payload diferente", response["ErrorMessage"])

    # ═══════════════════════════════════════════════════════════════
    # 6. PAGOS — ORDEN CANCELADA/FACTURADA
    # ═══════════════════════════════════════════════════════════════

    def test_payment_to_cancelled_order_rejected(self):
        """Pago a orden cancelada es rechazado."""
        order_id = self.create_order()["OrderId"]
        self.post("/API/Orders/Cancel", {"OrderIdentifier": {"OrderId": order_id}}, auth=True)
        payment = {"OrderIdentifier": {"OrderId": order_id}, "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-CANCEL"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("no pagable", response["ErrorMessage"])

    def test_payment_to_invoiced_order_rejected(self):
        """Pago a orden ya facturada es rechazado."""
        order_id = self.create_and_pay_order(total=10)
        self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": {"OrderId": order_id}}, auth=True)
        payment = {"OrderIdentifier": {"OrderId": order_id}, "Payment": {"TenderId": 1, "Amount": 5, "PaymentId": "PAY-INV-2"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("no pagable", response["ErrorMessage"])

    # ═══════════════════════════════════════════════════════════════
    # 7. FACTURACIÓN
    # ═══════════════════════════════════════════════════════════════

    def test_invoice_requires_full_payment(self):
        """No se puede facturar sin pagar completo."""
        order_id = self.create_order(total=15)["OrderId"]
        identifier = {"OrderId": order_id}
        self.post("/API/Orders/Payment/Add", {"OrderIdentifier": identifier,
            "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-PART"}}, auth=True)
        response = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertIn("saldo pendiente", response["ErrorMessage"])

    def test_invoice_cancelled_order_rejected(self):
        """No se puede facturar orden cancelada."""
        order_id = self.create_order()["OrderId"]
        self.post("/API/Orders/Cancel", {"OrderIdentifier": {"OrderId": order_id}}, auth=True)
        response = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": {"OrderId": order_id}}, auth=True)
        self.assertIn("cancelada", response["ErrorMessage"])

    def test_invoice_idempotent_on_already_invoiced(self):
        """Facturar orden ya facturada devuelve Duplicate."""
        order_id = self.create_and_pay_order(total=10)
        identifier = {"OrderId": order_id}
        first = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        second = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertTrue(second.get("Duplicate"))
        self.assertEqual(first["InvoiceNumber"], second["InvoiceNumber"])

    def test_invoice_nonexistent_order_fails(self):
        """Facturar orden inexistente falla."""
        response = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": {"OrderId": 99999}}, auth=True)
        self.assertIn("inexistente", response["ErrorMessage"])

    def test_invoice_assigns_unique_numbers(self):
        """Cada factura tiene un número único."""
        id1 = self.create_and_pay_order(total=5, marketplace_id="GINV0000000001", payment_id="P1")
        id2 = self.create_and_pay_order(total=5, marketplace_id="GINV0000000002", payment_id="P2")
        inv1 = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": {"OrderId": id1}}, auth=True)
        inv2 = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": {"OrderId": id2}}, auth=True)
        self.assertNotEqual(inv1["InvoiceNumber"], inv2["InvoiceNumber"])

    def test_invoiced_order_has_status_3(self):
        """Orden facturada tiene Status=3."""
        order_id = self.create_and_pay_order(total=10)
        self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": {"OrderId": order_id}}, auth=True)
        response = self.post("/API/Orders/Get", {"OrderIdentifier": {"OrderId": order_id}}, auth=True)
        self.assertEqual(response["Order"]["Status"], 3)

    # ═══════════════════════════════════════════════════════════════
    # 8. CANCELACIÓN DE ÓRDENES
    # ═══════════════════════════════════════════════════════════════

    def test_cancel_order_success(self):
        """Cancelar orden pendiente → Status=2."""
        order_id = self.create_order()["OrderId"]
        response = self.post("/API/Orders/Cancel", {"OrderIdentifier": {"OrderId": order_id}}, auth=True)
        self.assertEqual(response["Order"]["Status"], 2)
        self.assertEqual(response["ErrorMessage"], "")

    def test_cancel_nonexistent_order_fails(self):
        """Cancelar orden inexistente falla."""
        response = self.post("/API/Orders/Cancel", {"OrderIdentifier": {"OrderId": 99999}}, auth=True)
        self.assertIn("inexistente", response["ErrorMessage"])

    def test_cancel_already_invoiced_order_fails(self):
        """Cancelar orden ya facturada falla."""
        order_id = self.create_and_pay_order(total=10)
        self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": {"OrderId": order_id}}, auth=True)
        response = self.post("/API/Orders/Cancel", {"OrderIdentifier": {"OrderId": order_id}}, auth=True)
        self.assertIn("facturada", response["ErrorMessage"])

    def test_cancel_order_by_marketplace_id(self):
        """Cancelar usando MarketplaceOrderId."""
        self.create_order(marketplace_id="GCANCELTEST001")
        response = self.post("/API/Orders/Cancel", {"OrderIdentifier": {"MarketId": 77, "MarketplaceOrderId": "GCANCELTEST001"}}, auth=True)
        self.assertEqual(response["Order"]["Status"], 2)

    # ═══════════════════════════════════════════════════════════════
    # 9. CLIENTES
    # ═══════════════════════════════════════════════════════════════

    def test_create_customer_success(self):
        """Crear cliente válido."""
        response = self.post("/API/Customers/Create", {"Code": 101, "FiscalName": "Test SL",
            "CommercialName": "Test", "Overwrite": False}, auth=True)
        self.assertEqual(response["ErrorMessage"], "")
        self.assertIn("Customer", response)

    def test_create_customer_duplicate_without_overwrite(self):
        """Cliente duplicado sin Overwrite es rechazado."""
        self.post("/API/Customers/Create", {"Code": 201, "FiscalName": "Dup SL", "Overwrite": False}, auth=True)
        response = self.post("/API/Customers/Create", {"Code": 201, "FiscalName": "Dup SL 2", "Overwrite": False}, auth=True)
        self.assertIn("duplicado", response["ErrorMessage"])

    def test_create_customer_duplicate_with_overwrite(self):
        """Cliente duplicado con Overwrite=true reemplaza."""
        self.post("/API/Customers/Create", {"Code": 301, "FiscalName": "Original SL", "Overwrite": False}, auth=True)
        response = self.post("/API/Customers/Create", {"Code": 301, "FiscalName": "Updated SL", "Overwrite": True}, auth=True)
        self.assertEqual(response["ErrorMessage"], "")
        self.assertEqual(response["Customer"]["FiscalName"], "Updated SL")

    def test_create_customer_without_code_fails(self):
        """Cliente sin Code falla."""
        response = self.post("/API/Customers/Create", {"FiscalName": "No Code"}, auth=True)
        self.assertIn("Code", response["ErrorMessage"])

    def test_create_customer_without_fiscal_name_fails(self):
        """Cliente sin FiscalName falla."""
        response = self.post("/API/Customers/Create", {"Code": 401}, auth=True)
        self.assertIn("invalido", response["ErrorMessage"])

    def test_create_customer_negative_code_fails(self):
        """Cliente con Code negativo falla."""
        response = self.post("/API/Customers/Create", {"Code": -1, "FiscalName": "Neg"}, auth=True)
        self.assertIn("invalido", response["ErrorMessage"])

    # ═══════════════════════════════════════════════════════════════
    # 10. GET ORDER — IDENTIFICADORES
    # ═══════════════════════════════════════════════════════════════

    def test_get_order_by_order_id(self):
        """GetOrder por OrderId."""
        order_id = self.create_order()["OrderId"]
        response = self.post("/API/Orders/Get", {"OrderIdentifier": {"OrderId": order_id}}, auth=True)
        self.assertEqual(response["Order"]["OrderId"], order_id)
        self.assertEqual(response["ErrorMessage"], "")

    def test_get_order_by_marketplace_id(self):
        """GetOrder por MarketId + MarketplaceOrderId."""
        self.create_order(marketplace_id="GGETBYMKT001")
        response = self.post("/API/Orders/Get", {"OrderIdentifier": {"MarketId": 77, "MarketplaceOrderId": "GGETBYMKT001"}}, auth=True)
        self.assertEqual(response["Order"]["MarketplaceOrderId"], "GGETBYMKT001")

    def test_get_order_nonexistent_returns_error(self):
        """GetOrder para orden inexistente."""
        response = self.post("/API/Orders/Get", {"OrderIdentifier": {"OrderId": 99999}}, auth=True)
        self.assertIn("inexistente", response["ErrorMessage"])

    def test_get_order_without_identifier_returns_error(self):
        """GetOrder sin identificador."""
        response = self.post("/API/Orders/Get", {}, auth=True)
        self.assertIn("inexistente", response["ErrorMessage"])

    # ═══════════════════════════════════════════════════════════════
    # 11. FAULT INJECTION
    # ═══════════════════════════════════════════════════════════════

    def test_fault_http_status(self):
        """Fault http_status devuelve el código dado."""
        self.post("/__simulator/fault", {"Path": "/API/Orders/Get", "http_status": 503}, admin=True)
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/API/Orders/Get", {"OrderIdentifier": {"OrderId": 1}}, auth=True)
        self.assertEqual(ctx.exception.code, 503)

    def test_fault_remote_error(self):
        """Fault remote_error devuelve ErrorMessage sin error HTTP."""
        self.post("/__simulator/fault", {"Path": "/API/Orders/Get", "remote_error": "fallo funcional"}, admin=True)
        response = self.post("/API/Orders/Get", {}, auth=True)
        self.assertEqual(response["ErrorMessage"], "fallo funcional")

    def test_fault_invalid_json(self):
        """Fault invalid_json devuelve JSON inválido."""
        self.post("/__simulator/fault", {"Path": "/API/Orders/Get", "invalid_json": True}, admin=True)
        with self.assertRaises(json.JSONDecodeError):
            self.post("/API/Orders/Get", {}, auth=True)

    def test_fault_delay_ms(self):
        """Fault delay_ms causa delay antes de responder."""
        import time
        self.post("/__simulator/fault", {"Path": "/API/Orders/Get", "delay_ms": 500}, admin=True)
        start = time.monotonic()
        self.post("/API/Orders/Get", {"OrderIdentifier": {"OrderId": 99999}}, auth=True)
        elapsed = time.monotonic() - start
        self.assertGreaterEqual(elapsed, 0.4, "Delay should be at least 400ms")

    def test_fault_apply_then_disconnect(self):
        """Fault apply_then_disconnect aplica pero no responde."""
        self.post("/__simulator/fault", {"Path": "/API/Orders/Create", "apply_then_disconnect": True}, admin=True)
        with self.assertRaises(Exception):
            self.create_order(marketplace_id="GAMBIGUOUS002")
        # Verify order was created despite disconnect
        response = self.post("/API/Orders/Get", {"OrderIdentifier": {"MarketId": 77, "MarketplaceOrderId": "GAMBIGUOUS002"}}, auth=True)
        self.assertEqual(response["Order"]["MarketplaceOrderId"], "GAMBIGUOUS002")

    def test_faults_are_one_shot(self):
        """Faults se consumen después de un uso."""
        self.post("/__simulator/fault", {"Path": "/API/Orders/Get", "http_status": 503}, admin=True)
        with self.assertRaises(urllib.error.HTTPError):
            self.post("/API/Orders/Get", {}, auth=True)
        # Second call should succeed (fault consumed)
        response = self.post("/API/Orders/Get", {"OrderIdentifier": {"OrderId": 99999}}, auth=True)
        self.assertIn("inexistente", response["ErrorMessage"])

    def test_fault_on_write_path(self):
        """Fault en ruta de escritura (CreateOrder)."""
        self.post("/__simulator/fault", {"Path": "/API/Orders/Create", "http_status": 500}, admin=True)
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.create_order()
        self.assertEqual(ctx.exception.code, 500)

    def test_fault_on_payment_path(self):
        """Fault en ruta de pago."""
        order_id = self.create_order()["OrderId"]
        self.post("/__simulator/fault", {"Path": "/API/Orders/Payment/Add", "http_status": 502}, admin=True)
        payment = {"OrderIdentifier": {"OrderId": order_id}, "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-FAULT"}}
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertEqual(ctx.exception.code, 502)

    def test_fault_on_invoice_path(self):
        """Fault en ruta de facturación."""
        order_id = self.create_and_pay_order(total=10)
        self.post("/__simulator/fault", {"Path": "/API/Orders/Invoice", "http_status": 503}, admin=True)
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": {"OrderId": order_id}}, auth=True)
        self.assertEqual(ctx.exception.code, 503)

    def test_fault_on_login(self):
        """Fault en login."""
        self.post("/__simulator/reset", {}, admin=True)
        self.post("/__simulator/fault", {"Path": "/Auth/Login", "http_status": 500}, admin=True)
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/Auth/Login", {"Login": "local", "Password": "secret", "CodigoIntegrador": "SIM"})
        self.assertEqual(ctx.exception.code, 500)

    def test_fault_on_health(self):
        """Fault en health check."""
        self.post("/__simulator/fault", {"Path": "/Service/Health", "http_status": 503}, admin=True)
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/Service/Health", {})
        self.assertEqual(ctx.exception.code, 503)

    def test_fault_remote_error_on_create_order(self):
        """Fault remote_error en CreateOrder simula error funcional BDP."""
        self.post("/__simulator/fault", {"Path": "/API/Orders/Create", "remote_error": "[300035] serie no válida"}, admin=True)
        response = self.create_order()
        self.assertIn("300035", response["ErrorMessage"])

    def test_fault_remote_error_on_payment(self):
        """Fault remote_error en AddPayment."""
        order_id = self.create_order()["OrderId"]
        self.post("/__simulator/fault", {"Path": "/API/Orders/Payment/Add", "remote_error": "importe superior al saldo pendiente"}, admin=True)
        payment = {"OrderIdentifier": {"OrderId": order_id}, "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-ERR"}}
        response = self.post("/API/Orders/Payment/Add", payment, auth=True)
        self.assertIn("saldo pendiente", response["ErrorMessage"])

    # ═══════════════════════════════════════════════════════════════
    # 12. HISTORIAL Y ESTADO
    # ═══════════════════════════════════════════════════════════════

    def test_history_redacts_credentials_and_personal_data(self):
        self.post("/API/Customers/Create", {"Code": 101, "FiscalName": "Falso", "CommercialName": "Fixture",
            "MobilePhone": "123", "EMail": "x@example.invalid", "Overwrite": False}, auth=True)
        history = self.request("GET", "/__simulator/history", admin=True)["History"]
        serialized = json.dumps(history)
        self.assertNotIn("x@example.invalid", serialized)
        self.assertNotIn("123", serialized)
        self.assertNotIn(self.token, serialized)
        self.assertIn("[REDACTED]", serialized)

    def test_state_shows_orders(self):
        """State refleja órdenes creadas."""
        self.create_order(marketplace_id="GSTATE000001")
        state = self.request("GET", "/__simulator/state", admin=True)
        self.assertEqual(len(state["Orders"]), 1)

    def test_state_shows_customers(self):
        """State refleja clientes creados."""
        self.post("/API/Customers/Create", {"Code": 501, "FiscalName": "State Test", "Overwrite": False}, auth=True)
        state = self.request("GET", "/__simulator/state", admin=True)
        codes = [c["Code"] for c in state["Customers"]]
        self.assertIn(501, codes)

    def test_reset_clears_everything(self):
        """Reset limpia órdenes, clientes custom, pagos e historial."""
        self.create_order(marketplace_id="GRESET000001")
        self.post("/API/Customers/Create", {"Code": 601, "FiscalName": "Reset Test", "Overwrite": False}, auth=True)
        self.post("/__simulator/reset", {}, admin=True)
        state = self.request("GET", "/__simulator/state", admin=True)
        self.assertEqual(len(state["Orders"]), 0)
        # Fixtures should be reloaded
        self.assertGreater(len(state["Customers"]), 0)

    # ═══════════════════════════════════════════════════════════════
    # 13. FLUJO COMPLETO END-TO-END
    # ═══════════════════════════════════════════════════════════════

    def test_full_lifecycle_create_pay_invoice(self):
        """Flujo completo: crear → pagar → facturar → verificar."""
        # 1. Crear orden
        order_response = self.create_order(total=25)
        order_id = order_response["OrderId"]
        identifier = {"OrderId": order_id}
        self.assertGreater(order_id, 0)

        # 2. Verificar estado pendiente
        get_response = self.post("/API/Orders/Get", {"OrderIdentifier": identifier}, auth=True)
        self.assertEqual(get_response["Order"]["Status"], 0)
        self.assertEqual(get_response["Order"]["Payments"], [])

        # 3. Pago parcial
        partial = self.post("/API/Orders/Payment/Add", {"OrderIdentifier": identifier,
            "Payment": {"TenderId": 1, "Amount": 15, "PaymentId": "E2E-PARTIAL"}}, auth=True)
        self.assertEqual(partial["Balance"], 10.0)

        # 4. Segundo pago
        complete = self.post("/API/Orders/Payment/Add", {"OrderIdentifier": identifier,
            "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "E2E-REMAINING"}}, auth=True)
        self.assertEqual(complete["Balance"], 0.0)

        # 5. Facturar
        invoice = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertRegex(invoice["InvoiceNumber"], r"^SIM-\d{6}$")

        # 6. Verificar estado final
        final = self.post("/API/Orders/Get", {"OrderIdentifier": identifier}, auth=True)
        self.assertEqual(final["Order"]["Status"], 3)
        self.assertEqual(final["Order"]["InvoiceNumber"], invoice["InvoiceNumber"])
        self.assertEqual(len(final["Order"]["Payments"]), 2)

    def test_full_lifecycle_by_marketplace_id(self):
        """Flujo completo usando MarketplaceOrderId como identificador."""
        marketplace_id = "GE2EMKT00001"
        self.create_order(total=10, marketplace_id=marketplace_id)
        identifier = {"MarketId": 77, "MarketplaceOrderId": marketplace_id}
        self.post("/API/Orders/Payment/Add", {"OrderIdentifier": identifier,
            "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "E2E-MKT-PAY"}}, auth=True)
        invoice = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertIn("InvoiceNumber", invoice)

    def test_concurrent_duplicate_order_same_payload(self):
        """Dos requests idénticos concurrentes → misma orden."""
        import concurrent.futures
        payload = self.order_payload(marketplace_id="GCONCURRENT1")
        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
            futures = [executor.submit(self.post, "/API/Orders/Create", payload, auth=True) for _ in range(2)]
            results = [f.result() for f in concurrent.futures.as_completed(futures)]
        order_ids = {r["OrderId"] for r in results}
        self.assertEqual(len(order_ids), 1, "Concurrent identical requests should return same OrderId")

    # ═══════════════════════════════════════════════════════════════
    # 14. JSON Y VALIDACIÓN DE INPUT
    # ═══════════════════════════════════════════════════════════════

    def test_rejects_invalid_json_body(self):
        """Body con JSON inválido → 400."""
        request = urllib.request.Request(self.base + "/API/Orders/Create",
            data=b"{invalid json", method="POST",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {self.token}"})
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            urllib.request.urlopen(request, timeout=5)
        self.assertEqual(ctx.exception.code, 400)

    def test_rejects_non_dict_json_body(self):
        """Body que es array en vez de objeto → 400."""
        request = urllib.request.Request(self.base + "/API/Orders/Create",
            data=b"[1,2,3]", method="POST",
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {self.token}"})
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            urllib.request.urlopen(request, timeout=5)
        self.assertEqual(ctx.exception.code, 400)

    def test_unknown_route_returns_404(self):
        """Ruta no implementada → 404."""
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/API/Unknown/Route", {}, auth=True)
        self.assertEqual(ctx.exception.code, 404)

    # ═══════════════════════════════════════════════════════════════
    # 15. ESCENARIOS DE RECONCILIACIÓN
    # ═══════════════════════════════════════════════════════════════

    def test_applied_then_disconnected_can_be_reconciled(self):
        """Orden creada pero respuesta perdida → reconciliable por MarketplaceOrderId."""
        self.post("/__simulator/fault", {"Path": "/API/Orders/Create", "apply_then_disconnect": True}, admin=True)
        with self.assertRaises(Exception):
            self.create_order(marketplace_id="GAMBIGUOUS001")
        response = self.post("/API/Orders/Get", {"OrderIdentifier": {"MarketId": 77, "MarketplaceOrderId": "GAMBIGUOUS001"}}, auth=True)
        self.assertEqual(response["Order"]["MarketplaceOrderId"], "GAMBIGUOUS001")

    def test_payment_disconnect_can_be_reconciled(self):
        """Pago aplicado pero respuesta perdida → reconciliable verificando la orden."""
        order_id = self.create_order(total=10)["OrderId"]
        identifier = {"OrderId": order_id}
        self.post("/__simulator/fault", {"Path": "/API/Orders/Payment/Add", "apply_then_disconnect": True}, admin=True)
        payment = {"OrderIdentifier": identifier, "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-RECON"}}
        with self.assertRaises(Exception):
            self.post("/API/Orders/Payment/Add", payment, auth=True)
        # Verify payment was applied
        order = self.post("/API/Orders/Get", {"OrderIdentifier": identifier}, auth=True)
        self.assertEqual(len(order["Order"]["Payments"]), 1)
        self.assertEqual(order["Order"]["Payments"][0]["PaymentId"], "PAY-RECON")


if __name__ == "__main__":
    unittest.main()
