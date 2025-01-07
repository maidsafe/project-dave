import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useToast } from 'primevue/usetoast';

export async function updater() {
  const toast = useToast();
  try {
    console.log('checking for updates...');
    toast.add({
      severity: 'info',
      summary: 'Update check',
      detail: 'Checking for updates...',
      life: 6000,
    });
    console.log('toast', toast);
    const update = await check();
    console.log('update', update);
    if (update) {
      console.log(
        `found update ${update.version} from ${update.date} with notes ${update.body}`,
      );
      toast.add({
        severity: 'info',
        summary: 'Update found',
        detail: 'Update found, downloading now...',
        life: 6000,
      });
      let downloaded = 0;
      let contentLength = 0;
      // alternatively we could also call update.download() and update.install() separately
      await update.downloadAndInstall(event => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength as number;
            console.log(
              `started downloading ${event.data.contentLength} bytes`,
            );
            toast.add({
              severity: 'info',
              summary: 'Update downloading',
              detail: 'Update download started...',
              life: 6000,
            });
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            console.log(`downloaded ${downloaded} from ${contentLength}`);
            toast.add({
              severity: 'info',
              summary: 'Update status',
              detail: `downloaded ${downloaded} from ${contentLength}`,
              life: 6000,
            });
            break;
          case 'Finished':
            console.log('download finished');
            toast.add({
              severity: 'info',
              summary: 'Update download finished',
              detail: 'Update download finished...',
              life: 6000,
            });
            break;
        }
      });

      console.log('update installed');
      toast.add({
        severity: 'info',
        summary: 'Update installed',
        detail: 'Update installed, relaunching now...',
        life: 6000,
      });
      await relaunch();
    } else {
      console.log('no update found');
      toast.add({
        severity: 'info',
        summary: 'No update found',
        detail: 'No update found, will check again on next launch...',
        life: 6000,
      });
    }
  } catch (error) {
    console.log(error);
    toast.add({
      severity: 'error',
      summary: 'There was an error updating',
      detail: error,
      life: 6000,
    });
  }
}
