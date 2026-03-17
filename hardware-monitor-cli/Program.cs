using System.Text.Json;
using System.Text.Json.Serialization;
using LibreHardwareMonitor.Hardware;
using System.Net;
using System.Text;
using System.Runtime.InteropServices;

// Запуск: либо как Windows Service, либо как консольное приложение
var builder = Host.CreateDefaultBuilder(args)
    .UseWindowsService(options => { options.ServiceName = "HardwareMonitorCli"; })
    .ConfigureServices(services => { services.AddHostedService<HardwareMonitorService>(); });

await builder.Build().RunAsync();

// ─── Hosted Service ────────────────────────────────────────────────────────────

public class HardwareMonitorService : BackgroundService
{
    private readonly Computer _computer;
    private HttpListener? _listener;

    public HardwareMonitorService()
    {
        _computer = new Computer
        {
            IsCpuEnabled = true,
            IsGpuEnabled = true,
            IsMemoryEnabled = true,
            IsStorageEnabled = true,
            IsMotherboardEnabled = true
        };
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        _computer.Open();

        _listener = new HttpListener();
        _listener.Prefixes.Add("http://localhost:8765/");

        try
        {
            _listener.Start();
            Console.Error.WriteLine("LibreHardwareMonitor service started on http://localhost:8765");
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Failed to start HTTP server: {ex.Message}");
            return;
        }

        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                var context = await _listener.GetContextAsync().WaitAsync(stoppingToken);
                _ = Task.Run(() => HandleRequest(context, stoppingToken), stoppingToken);
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error accepting request: {ex.Message}");
            }
        }
    }

    private async Task HandleRequest(HttpListenerContext context, CancellationToken stoppingToken)
    {
        var request = context.Request;
        var response = context.Response;

        try
        {
            if (request.Url?.AbsolutePath == "/shutdown")
            {
                Console.Error.WriteLine("Shutdown request received");
                var shutdownMsg = Encoding.UTF8.GetBytes("{\"status\":\"ok\"}");
                response.ContentType = "application/json";
                response.ContentLength64 = shutdownMsg.Length;
                response.StatusCode = 200;
                await response.OutputStream.WriteAsync(shutdownMsg, 0, shutdownMsg.Length, stoppingToken);
                response.OutputStream.Close();
                // Graceful stop через StopAsync — не нужен принудительный выход
                return;
            }

            var metrics = CollectMetrics();
            var json = JsonSerializer.Serialize(metrics);
            var buffer = Encoding.UTF8.GetBytes(json);

            response.ContentType = "application/json";
            response.ContentLength64 = buffer.Length;
            response.StatusCode = 200;
            await response.OutputStream.WriteAsync(buffer, 0, buffer.Length, stoppingToken);
            response.OutputStream.Close();
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error handling request: {ex.Message}");
            try
            {
                response.OutputStream.Close();
            }
            catch
            {
            }
        }
    }

    private SystemMetrics CollectMetrics()
    {
        float? cpuTemp = null;
        float? gpuTemp = null;
        float? gpuUsage = null;
        float cpuUsageTotal = 0;

        foreach (var hardware in _computer.Hardware)
        {
            hardware.Update();

            foreach (var sensor in hardware.Sensors)
            {
                if (!sensor.Value.HasValue) continue;

                var value = sensor.Value.Value;
                var sensorName = sensor.Name?.ToLower() ?? "";

                switch (sensor.SensorType)
                {
                    case SensorType.Temperature:
                        if (value < -30 || value > 200) continue;
                        switch (hardware.HardwareType)
                        {
                            case HardwareType.Cpu:
                                if (sensorName.Contains("package") || sensorName.Contains("total"))
                                    cpuTemp = value;
                                else if (!cpuTemp.HasValue)
                                    cpuTemp = value;
                                break;
                            case HardwareType.GpuAmd:
                            case HardwareType.GpuNvidia:
                            case HardwareType.GpuIntel:
                                if (sensorName.Contains("core") && !sensorName.Contains("memory"))
                                    gpuTemp = value;
                                else if (!gpuTemp.HasValue && !sensorName.Contains("memory") &&
                                         !sensorName.Contains("junction"))
                                    gpuTemp = value;
                                break;
                        }

                        break;

                    case SensorType.Load:
                        if (hardware.HardwareType == HardwareType.Cpu)
                        {
                            if (sensorName.Contains("total"))
                                cpuUsageTotal = value;
                        }
                        else if (hardware.HardwareType is HardwareType.GpuNvidia
                                 or HardwareType.GpuAmd
                                 or HardwareType.GpuIntel)
                        {
                            if (sensorName.Contains("core") && !sensorName.Contains("memory")
                                                            && !sensorName.Contains("video"))
                                gpuUsage = value;
                        }

                        break;
                }
            }
        }

        // Fallback: температура CPU с материнской платы
        if (!cpuTemp.HasValue)
        {
            foreach (var hardware in _computer.Hardware)
            {
                if (hardware.HardwareType != HardwareType.Motherboard) continue;
                hardware.Update();
                foreach (var sensor in hardware.Sensors)
                {
                    if (sensor.SensorType != SensorType.Temperature || !sensor.Value.HasValue) continue;
                    var value = sensor.Value.Value;
                    var name = sensor.Name?.ToLower() ?? "";
                    if (value >= -30 && value <= 200 &&
                        (name.Contains("cpu") || name.Contains("package") ||
                         name.Contains("tctl") || name.Contains("tdie") || name.Contains("processor")))
                    {
                        cpuTemp = value;
                        break;
                    }
                }

                if (cpuTemp.HasValue) break;
            }
        }

        var disks = new List<DiskUsage>();
        foreach (var drive in DriveInfo.GetDrives())
        {
            try
            {
                if (!drive.IsReady || drive.DriveType != DriveType.Fixed) continue;
                var total = (ulong)drive.TotalSize;
                var available = (ulong)drive.AvailableFreeSpace;
                var used = total - available;
                disks.Add(new DiskUsage
                {
                    Name = drive.Name,
                    MountPoint = drive.RootDirectory.FullName,
                    TotalSpace = total,
                    AvailableSpace = available,
                    UsedSpace = used,
                    UsagePercent = total > 0 ? (float)used / total * 100 : 0
                });
            }
            catch
            {
            }
        }

        var (memTotal, memUsed) = MemoryHelper.GetMemoryInfo();

        return new SystemMetrics
        {
            CpuUsage = cpuUsageTotal,
            CpuTemperature = cpuTemp,
            GpuUsage = gpuUsage,
            GpuTemperature = gpuTemp,
            MemoryTotal = memTotal,
            MemoryUsed = memUsed,
            Disks = disks
        };
    }

    public override async Task StopAsync(CancellationToken cancellationToken)
    {
        _listener?.Stop();
        _computer.Close();
        Console.Error.WriteLine("LibreHardwareMonitor service stopped.");
        await base.StopAsync(cancellationToken);
    }
}

// ─── Модели ────────────────────────────────────────────────────────────────────

class DiskUsage
{
    [JsonPropertyName("name")] public string Name { get; set; } = "";
    [JsonPropertyName("mount_point")] public string MountPoint { get; set; } = "";
    [JsonPropertyName("total_space")] public ulong TotalSpace { get; set; }
    [JsonPropertyName("available_space")] public ulong AvailableSpace { get; set; }
    [JsonPropertyName("used_space")] public ulong UsedSpace { get; set; }
    [JsonPropertyName("usage_percent")] public float UsagePercent { get; set; }
}

class SystemMetrics
{
    [JsonPropertyName("cpu_usage")] public float CpuUsage { get; set; }
    [JsonPropertyName("cpu_temperature")] public float? CpuTemperature { get; set; }
    [JsonPropertyName("gpu_usage")] public float? GpuUsage { get; set; }
    [JsonPropertyName("gpu_temperature")] public float? GpuTemperature { get; set; }
    [JsonPropertyName("memory_total")] public ulong MemoryTotal { get; set; }
    [JsonPropertyName("memory_used")] public ulong MemoryUsed { get; set; }
    [JsonPropertyName("disks")] public List<DiskUsage> Disks { get; set; } = new();
}

[StructLayout(LayoutKind.Sequential)]
struct MEMORYSTATUSEX
{
    public uint dwLength;
    public uint dwMemoryLoad;
    public ulong ullTotalPhys;
    public ulong ullAvailPhys;
    public ulong ullTotalPageFile;
    public ulong ullAvailPageFile;
    public ulong ullTotalVirtual;
    public ulong ullAvailVirtual;
    public ulong ullAvailExtendedVirtual;
}

static class NativeMethods
{
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GlobalMemoryStatusEx(ref MEMORYSTATUSEX lpBuffer);
}

static class MemoryHelper
{
    public static (ulong total, ulong used) GetMemoryInfo()
    {
        var memStatus = new MEMORYSTATUSEX { dwLength = (uint)Marshal.SizeOf(typeof(MEMORYSTATUSEX)) };
        if (NativeMethods.GlobalMemoryStatusEx(ref memStatus))
            return (memStatus.ullTotalPhys, memStatus.ullTotalPhys - memStatus.ullAvailPhys);
        return (0, 0);
    }
}