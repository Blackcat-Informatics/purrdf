from pathlib import Path
import difflib
stage=Path('.stage/sparql-eval-complete-bounded-workspace'); changes={}
def load(path):
    s=Path(path).read_text(); changes[path]=[s,s]; return s
def save(path,s): changes[path][1]=s
path='crates/sparql-eval/src/eval.rs'; s=load(path)
s=s.replace('pub standpoint_predicates: Option<StandpointPredicates>,','pub standpoint_predicates: Option<std::sync::Arc<StandpointPredicates>>,',1)
s=s.replace('pub loss_vocabulary: Option<LossVocabulary>,','pub loss_vocabulary: Option<std::sync::Arc<LossVocabulary>>,',1)
s=s.replace('''    pub fn with_standpoint_predicates(mut self, predicates: StandpointPredicates) -> Self {
        self.standpoint_predicates = Some(predicates);
        self
    }''','''    pub fn with_standpoint_predicates(self, predicates: StandpointPredicates) -> Self {
        self.with_shared_standpoint_predicates(std::sync::Arc::new(predicates))
    }

    /// Share the caller's immutable registration without new execution storage.
    pub(crate) fn with_shared_standpoint_predicates(mut self, predicates: std::sync::Arc<StandpointPredicates>) -> Self {
        self.standpoint_predicates = Some(predicates);
        self
    }''',1)
s=s.replace('''    pub fn with_loss_vocabulary(mut self, vocab: LossVocabulary) -> Self {
        self.loss_vocabulary = Some(vocab);
        self
    }''','''    pub fn with_loss_vocabulary(self, vocab: LossVocabulary) -> Self {
        self.with_shared_loss_vocabulary(std::sync::Arc::new(vocab))
    }

    /// Share the existing vocabulary registration through worker/context copies.
    pub(crate) fn with_shared_loss_vocabulary(mut self, vocab: std::sync::Arc<LossVocabulary>) -> Self {
        self.loss_vocabulary = Some(vocab);
        self
    }''',1)
save(path,s)
path='crates/sparql-eval/src/engine.rs'; s=load(path)
s=s.replace('standpoint_predicates: Option<StandpointPredicates>,','standpoint_predicates: Option<std::sync::Arc<StandpointPredicates>>,',1)
s=s.replace('loss_vocabulary: Option<LossVocabulary>,','loss_vocabulary: Option<std::sync::Arc<LossVocabulary>>,',1)
s=s.replace('self.standpoint_predicates = Some(predicates);','self.standpoint_predicates = Some(std::sync::Arc::new(predicates));',1)
s=s.replace('self.loss_vocabulary = Some(vocab);','self.loss_vocabulary = Some(std::sync::Arc::new(vocab));',1)
s=s.replace('ctx = ctx.with_standpoint_predicates(predicates.clone());','ctx = ctx.with_shared_standpoint_predicates(std::sync::Arc::clone(predicates));')
s=s.replace('ctx = ctx.with_loss_vocabulary(vocab.clone());','ctx = ctx.with_shared_loss_vocabulary(std::sync::Arc::clone(vocab));')
save(path,s)
path='crates/sparql-eval/src/property_fn_eval.rs'; s=load(path)
s=s.replace('standpoint_predicates: Option<crate::eval::StandpointPredicates>,','standpoint_predicates: Option<std::sync::Arc<crate::eval::StandpointPredicates>>,',1)
s=s.replace('loss_vocabulary: Option<crate::eval::LossVocabulary>,','loss_vocabulary: Option<std::sync::Arc<crate::eval::LossVocabulary>>,',1)
s=s.replace('ctx = ctx.with_standpoint_predicates(predicates.clone());','ctx = ctx.with_shared_standpoint_predicates(std::sync::Arc::clone(predicates));')
s=s.replace('ctx = ctx.with_loss_vocabulary(vocabulary.clone());','ctx = ctx.with_shared_loss_vocabulary(std::sync::Arc::clone(vocabulary));')
save(path,s)
path='crates/sparql-eval/src/update.rs'; s=load(path)
s=s.replace("standpoint_predicates: Option<&'e StandpointPredicates>,","standpoint_predicates: Option<&'e std::sync::Arc<StandpointPredicates>>,")
s=s.replace('ctx = ctx.with_standpoint_predicates(preds.clone());','ctx = ctx.with_shared_standpoint_predicates(std::sync::Arc::clone(preds));')
save(path,s)
out=stage/'native-shared-config-postimages'; out.mkdir(exist_ok=True); patch=''
for path,(old,new) in changes.items():
    target=out/path; target.parent.mkdir(parents=True,exist_ok=True); target.write_text(new)
    patch+=''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='a/'+path,tofile='b/'+path))
(stage/'native-shared-config-owner-draft.patch').write_text(patch)
