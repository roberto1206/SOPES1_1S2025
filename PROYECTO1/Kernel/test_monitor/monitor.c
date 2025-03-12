#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/mm.h>
#include <linux/sched.h>
#include <linux/jiffies.h>
#include <linux/sysinfo.h>
#include <linux/fs.h>
#include <linux/time.h>
#include <linux/uaccess.h>
#include <linux/slab.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("Roberto");
MODULE_DESCRIPTION("Modulo para capturar informacion de memoria y CPU en JSON");
MODULE_VERSION("1.0");

#define PROC_NAME "sysinfo_202201724"

static unsigned long prev_idle = 0;
static unsigned long prev_total = 0;
static ktime_t last_time = 0;

static int get_cpu_usage(void) {
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

static int sysinfo_show(struct seq_file *m, void *v) {
    struct sysinfo si;
    unsigned long usedram, freeram, cachedram;
    int cpu_usage;

    si_meminfo(&si);
    cachedram = si.sharedram + si.bufferram; // Incluir sharedram y bufferram como cache
    freeram = si.freeram + cachedram; // La memoria libre es la freeram mas el cache
    usedram = si.totalram - freeram;

    cpu_usage = get_cpu_usage();

    seq_printf(m, "{\n");
    seq_printf(m, "  \"Total_RAM_KB\": %lu,\n", si.totalram * si.mem_unit / 1024);
    seq_printf(m, "  \"Free_RAM_KB\": %lu,\n", freeram * si.mem_unit / 1024);
    seq_printf(m, "  \"Used_RAM_KB\": %lu,\n", usedram * si.mem_unit / 1024);
    seq_printf(m, "  \"CPU_Usage_Percentage\": %d\n", cpu_usage);
    seq_printf(m, "}\n");

    return 0;
}

static int sysinfo_open(struct inode *inode, struct file *file) {
    return single_open(file, sysinfo_show, NULL);
}

static const struct proc_ops sysinfo_ops = {
    .proc_open = sysinfo_open,
    .proc_read = seq_read,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

static int __init sysinfo_init(void) {
    proc_create(PROC_NAME, 0, NULL, &sysinfo_ops);
    printk(KERN_INFO "sysinfo_202201724 module loaded\n");
    return 0;
}

static void __exit sysinfo_exit(void) {
    remove_proc_entry(PROC_NAME, NULL);
    printk(KERN_INFO "sysinfo_202201724 module unloaded\n");
}

module_init(sysinfo_init);
module_exit(sysinfo_exit);