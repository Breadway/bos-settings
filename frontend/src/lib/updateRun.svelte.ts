/** Keep real update progress visible if the user switches pages. */
export const updateRun = $state({ busy:false, log:[] as string[], steps:[] as {label:string; state:"waiting"|"running"|"done"|"failed"}[], message:"" });
