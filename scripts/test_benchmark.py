import unittest

from benchmark import parse_vm_hwm_kb, stats


class BenchmarkHelpersTest(unittest.TestCase):
    def test_vm_hwm_extracts_linux_status_kilobytes(self):
        self.assertEqual(parse_vm_hwm_kb('Name:\tentail\nVmRSS:\t100 kB\nVmHWM:\t2480 kB\n'), 2480)

    def test_vm_hwm_missing_is_unavailable(self):
        self.assertIsNone(parse_vm_hwm_kb('Name:\tentail\nVmRSS:\t100 kB\n'))

    def test_stats_preserve_detached_native_values(self):
        report = {'results': [{'obligation_id': 'o1', 'status': 'satisfied', 'elapsed_ms': 7,
                               'statistics': {'values': {'assertion_count': '3', 'memory': '10'}}}]}
        result = stats(report)
        self.assertEqual(result['native_stats_obligations'][0]['values'],
                         {'assertion_count': '3', 'memory': '10'})

    def test_cached_result_does_not_reuse_old_native_statistics(self):
        report = {'results': [{'obligation_id': 'o1', 'status': 'satisfied', 'statistics': None}]}
        result = stats(report)
        self.assertEqual(result['native_stats_obligations'], [])


if __name__ == '__main__':
    unittest.main()
