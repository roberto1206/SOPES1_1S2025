#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/sched/signal.h>
#include <linux/slab.h>
#include <linux/mm.h>
#include <linux/time.h>
#include <linux/cpumask.h>
#include <linux/jiffies.h>
#include <linux/sched/mm.h>
#include <linux/sched/task.h>
#include <linux/math64.h>
#include <linux/uaccess.h>
#include <linux/sysinfo.h>
#include <linux/fs.h>

#define PROC_NAME "sysinfo_202201724"
#define CONTAINER_PREFIX "stress" // Prefijo para filtrar contenedores

// Estructura para almacenar información de I/O
struct io_info {
    unsigned long long read_bytes;
    unsigned long long write_bytes;
};

// Variables para rastrear el uso de CPU del sistema
static unsigned long prev_idle = 0;
static unsigned long prev_total = 0;
static ktime_t last_time = 0;

// Obtener estadísticas de I/O
static void get_io_stats(struct task_struct *task, struct io_info *io)
{
    char buffer[4096];
    char filename[256];
    struct file *f;
    int ret;
    loff_t pos = 0;
    
    // Inicializar valores
    io->read_bytes = 0;
    io->write_bytes = 0;
    
    // En sistemas modernos, la información de I/O se accede desde /proc/{pid}/io
    snprintf(filename, sizeof(filename), "/proc/%d/io", task->pid);
    
    f = filp_open(filename, O_RDONLY, 0);
    if (IS_ERR(f)) {
        return;
    }
    
    ret = kernel_read(f, buffer, sizeof(buffer) - 1, &pos);
    filp_close(f, NULL);
    
    if (ret <= 0) {
        return;
    }
    
    buffer[ret] = '\0';
    
    // Analizar el contenido del archivo para obtener read_bytes y write_bytes
    char *p = buffer;
    char *line;
    
    while ((line = strsep(&p, "\n"))) {
        unsigned long long value;
        
        if (sscanf(line, "read_bytes: %llu", &value) == 1) {
            io->read_bytes = value;
        } else if (sscanf(line, "write_bytes: %llu", &value) == 1) {
            io->write_bytes = value;
        }
    }
}

// Obtener el uso de disco
static unsigned long get_disk_usage(struct task_struct *task)
{
    struct mm_struct *mm = task->mm;
    unsigned long disk_usage = 0;
    
    if (mm) {
        // Aproximar el uso de disco como la memoria RSS
        disk_usage = get_mm_rss(mm);
        
        // En kernels modernos, swap_usage no está disponible directamente
        
        disk_usage <<= PAGE_SHIFT;  // Convertir páginas a bytes
        disk_usage >>= 10;         // Convertir a KB
    }
    
    return disk_usage;
}

// Función para obtener el porcentaje de CPU de un proceso
static unsigned int get_process_cpu_usage(struct task_struct *task)
{
    unsigned long total, seconds;
    unsigned int cpu_usage = 0;
    unsigned long user_time, system_time;
    
    if (!task)
        return 0;
    
    user_time = task->utime;
    system_time = task->stime;
    
    total = user_time + system_time;
    
    // Calculamos el tiempo transcurrido desde el inicio del proceso
    seconds = div_u64(ktime_get_ns() - task->start_time, NSEC_PER_SEC);
    
    if (seconds > 0) {
        // El tiempo de CPU es en jiffies, por lo que lo convertimos a segundos
        // y calculamos como porcentaje del tiempo total
        u64 cpu_time_sec = div_u64(total, HZ);
        cpu_usage = div_u64(cpu_time_sec * 100, seconds);
        
        // Limitar a 100% para un único núcleo
        if (cpu_usage > 100)
            cpu_usage = 100;
    }
    
    return cpu_usage;
}

// Función para obtener el porcentaje de memoria de un proceso
static unsigned int get_memory_usage(struct task_struct *task)
{
    unsigned long total_ram, rss = 0;
    unsigned int mem_usage = 0;
    struct mm_struct *mm;
    
    if (!task)
        return 0;
    
    mm = task->mm;
    if (!mm)
        return 0;
    
    // Obtenemos el RSS en bytes
    rss = get_mm_rss(mm) << PAGE_SHIFT;
    
    // Obtenemos la RAM total en bytes
    total_ram = totalram_pages() << PAGE_SHIFT;
    
    if (total_ram > 0) {
        // Calculamos el porcentaje
        mem_usage = div_u64((u64)rss * 100, total_ram);
    }
    
    return mem_usage;
}

// Función para obtener la línea de comando de un proceso
static int get_process_cmdline(struct task_struct *task, char *buffer, int buffer_size)
{
    int ret = 0;
    struct mm_struct *mm;
    
    if (!task || !buffer || buffer_size <= 0)
        return 0;
    
    mm = get_task_mm(task);
    if (!mm) {
        // Si no hay mm, simplemente copiamos el nombre del proceso
        strncpy(buffer, task->comm, buffer_size - 1);
        buffer[buffer_size - 1] = '\0';
        return strlen(buffer);
    }
    
    down_read(&mm->mmap_lock);
    
    if (mm->arg_end > mm->arg_start) {
        unsigned long len = mm->arg_end - mm->arg_start;
        
        if (len > buffer_size - 1)
            len = buffer_size - 1;
            
        ret = access_process_vm(task, mm->arg_start, buffer, len, FOLL_FORCE);
        
        // Reemplazar caracteres nulos por espacios para mejor visualización
        if (ret > 0) {
            int i;
            for (i = 0; i < ret - 1; i++) {
                if (buffer[i] == '\0')
                    buffer[i] = ' ';
            }
            buffer[ret] = '\0';
        }
    }
    
    up_read(&mm->mmap_lock);
    mmput(mm);
    
    // Si no pudimos obtener la línea de comandos, usar el nombre del proceso
    if (ret <= 0) {
        strncpy(buffer, task->comm, buffer_size - 1);
        buffer[buffer_size - 1] = '\0';
        ret = strlen(buffer);
    }
    
    return ret;
}

// Función para obtener el uso de CPU del sistema
static int get_system_cpu_usage(void) {
    struct file *file;
    char *buffer;
    char *ptr;
    unsigned long user, nice, system, idle, iowait, irq, softirq, steal;
    unsigned long total_time, idle_time;
    unsigned long diff_idle, diff_total;
    int cpu_percentage = 0;
    ktime_t now;
    ssize_t bytes_read;
    loff_t pos = 0;

    buffer = kmalloc(256, GFP_KERNEL);
    if (!buffer)
        return 0;

    file = filp_open("/proc/stat", O_RDONLY, 0);
    if (IS_ERR(file)) {
        printk(KERN_ERR "Error al abrir /proc/stat\n");
        kfree(buffer);
        return 0;
    }

    bytes_read = kernel_read(file, buffer, 255, &pos);
    filp_close(file, NULL);

    if (bytes_read <= 0) {
        printk(KERN_ERR "Error al leer /proc/stat\n");
        kfree(buffer);
        return 0;
    }
    buffer[bytes_read] = '\0';

    ptr = buffer;
    while (*ptr != ' ') ptr++;
    ptr++;

    sscanf(ptr, "%lu %lu %lu %lu %lu %lu %lu %lu",
           &user, &nice, &system, &idle, &iowait, &irq, &softirq, &steal);

    kfree(buffer);

    idle_time = idle + iowait;
    total_time = user + nice + system + idle_time + irq + softirq + steal;
    now = ktime_get();

    if (prev_total == 0 || last_time == 0) {
        prev_idle = idle_time;
        prev_total = total_time;
        last_time = now;
        return 0;
    }

    if (ktime_to_ms(ktime_sub(now, last_time)) > 100) {
        diff_idle = idle_time - prev_idle;
        diff_total = total_time - prev_total;

        if (diff_total > 0) {
            cpu_percentage = (1000 * (diff_total - diff_idle) / diff_total + 5) / 10;
            if (cpu_percentage > 100)
                cpu_percentage = 100;
        }

        prev_idle = idle_time;
        prev_total = total_time;
        last_time = now;
    }

    return cpu_percentage;
}

static int containers_show(struct seq_file *m, void *v)
{
    struct task_struct *task;
    struct mm_struct *mm;
    unsigned long rss, vm_size;
    char cmdline[256];
    struct io_info io;
    unsigned int cpu_usage, mem_usage;
    unsigned long disk_usage;
    struct sysinfo si;
    unsigned long usedram, freeram, cachedram;
    int system_cpu_usage;

    // Obtener información general del sistema
    si_meminfo(&si);
    cachedram = si.sharedram + si.bufferram; // Incluir sharedram y bufferram como cache
    freeram = si.freeram + cachedram; // La memoria libre es la freeram mas el cache
    usedram = si.totalram - freeram;
    system_cpu_usage = get_system_cpu_usage();

    // Imprimir información del sistema
    seq_printf(m, "{\n");
    seq_printf(m, "  \"system\": {\n");
    seq_printf(m, "    \"Total_RAM_KB\": %lu,\n", si.totalram * si.mem_unit / 1024);
    seq_printf(m, "    \"Free_RAM_KB\": %lu,\n", freeram * si.mem_unit / 1024);
    seq_printf(m, "    \"Used_RAM_KB\": %lu,\n", usedram * si.mem_unit / 1024);
    seq_printf(m, "    \"CPU_Usage_Percentage\": %d\n", system_cpu_usage);
    seq_printf(m, "  },\n");
    
    // Imprimir información de contenedores
    seq_printf(m, "  \"containers\": [\n");
    int first = 1;
    
    // Recorrer todos los procesos
    for_each_process(task) {
        if (strnstr(task->comm, CONTAINER_PREFIX, TASK_COMM_LEN)) { // Filtrar contenedores
            if (!first) seq_puts(m, ",\n");
            first = 0;
            
            mm = task->mm;
            rss = (mm) ? get_mm_rss(mm) << PAGE_SHIFT : 0;
            vm_size = (mm) ? mm->total_vm << PAGE_SHIFT : 0;
            
            get_process_cmdline(task, cmdline, sizeof(cmdline));
            get_io_stats(task, &io);
            cpu_usage = get_process_cpu_usage(task);
            mem_usage = get_memory_usage(task);
            disk_usage = get_disk_usage(task);
            
            seq_printf(m, "    {\n");
            seq_printf(m, "      \"pid\": %d,\n", task->pid);
            seq_printf(m, "      \"name\": \"%s\",\n", task->comm);
            seq_printf(m, "      \"cmdline\": \"%s\",\n", cmdline);
            seq_printf(m, "      \"memory_rss\": %lu,\n", rss);
            seq_printf(m, "      \"memory_percent\": %u,\n", mem_usage);
            seq_printf(m, "      \"virtual_memory\": %lu,\n", vm_size);
            seq_printf(m, "      \"cpu_percent\": %u,\n", cpu_usage);
            seq_printf(m, "      \"disk_usage\": %lu,\n", disk_usage);
            seq_printf(m, "      \"io_read_bytes\": %llu,\n", io.read_bytes);
            seq_printf(m, "      \"io_write_bytes\": %llu\n", io.write_bytes);
            seq_printf(m, "    }");
        }
    }
    seq_puts(m, "\n  ]\n}\n");
    return 0;
}

static int sysinfo_open(struct inode *inode, struct file *file)
{
    return single_open(file, containers_show, NULL);
}

static const struct proc_ops sysinfo_ops = {
    .proc_open = sysinfo_open,
    .proc_read = seq_read,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

static int __init sysinfo_init(void)
{
    struct proc_dir_entry *proc_entry;
    proc_entry = proc_create(PROC_NAME, 0, NULL, &sysinfo_ops);
    if (!proc_entry) {
        printk(KERN_ERR "Error al crear la entrada en /proc/%s\n", PROC_NAME);
        return -ENOMEM;
    }
    
    printk(KERN_INFO "Módulo sysinfo cargado. Monitorizando procesos '%s*' y rendimiento del sistema\n", CONTAINER_PREFIX);
    return 0;
}

static void __exit sysinfo_exit(void)
{
    remove_proc_entry(PROC_NAME, NULL);
    printk(KERN_INFO "Módulo sysinfo eliminado.\n");
}

module_init(sysinfo_init);
module_exit(sysinfo_exit);
MODULE_LICENSE("GPL");
MODULE_AUTHOR("Roberto");
MODULE_DESCRIPTION("Módulo para capturar información detallada de contenedores y rendimiento del sistema");
MODULE_VERSION("1.0");