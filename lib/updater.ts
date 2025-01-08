import { check } from '@tauri-apps/plugin-updater';
import { useToast } from 'primevue/usetoast';

export async function updater() {
  const toast = useToast();
  try {
    console.log('checking for updates...');
    const update = await check();
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
      await update.downloadAndInstall(event => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength as number;
            console.log(
              `started downloading ${event.data.contentLength} bytes`,
            );
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            console.log(`downloaded ${downloaded} from ${contentLength}`);
            break;
          case 'Finished':
            console.log('download finished');
            toast.add({
              severity: 'info',
              summary: 'Update downloaded',
              detail: 'Update downloaded, will install on next launch...',
              life: 6000,
            });
            break;
        }
      });
      console.log('update installed');
    } else {
      console.log('no update found');
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
