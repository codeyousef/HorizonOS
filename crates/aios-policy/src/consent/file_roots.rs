//! Exact native root enrollment. A client boolean cannot construct a decision.
//! This grants scoped root access only, never execution of a file mutation.
use super::*;
#[derive(Clone,PartialEq,Eq,Serialize)]
pub struct FileRoot { pub root_id:String,pub display_path:String,pub identity_sha256:String }
#[derive(Clone,Copy,PartialEq,Eq,Serialize)]
#[serde(rename_all="snake_case")]
pub enum RootAccess { Metadata,Content,Mutation }
pub struct FileRootsProposal {
    intent:Intent,pub(super) desktop:NativeDesktop,roots:Vec<FileRoot>,access:Vec<RootAccess>,
    revision:String,incarnation:Uuid,nonce:Uuid,issued_ms:u64,expires_ms:u64,pub(super) digest:String,
}
pub struct NativeFileRootsDecision { proposal:FileRootsProposal,cancelled:Arc<AtomicBool> }
pub struct FileRootsConfirmation { native:NativeConfirmation }
impl Policy {
    pub fn propose_file_roots(&self,intent:Intent,desktop:NativeDesktop,roots:Vec<FileRoot>,access:Vec<RootAccess>,expiry_ms:u64)->Result<FileRootsProposal>{
        intent.subject.validate()?;
        if intent.mode!=Mode::Act||intent.subject.uid==0||intent.subject.uid!=desktop.uid||intent.subject.boot_id!=desktop.boot_id||desktop.boot_id!=self.boot_id{return Err(ErrorCode::PermissionDenied);}
        if !session_id(&desktop.session_id)||!hash(&desktop.identity_sha256)||!desktop.socket_name.starts_with("wayland-")
            || !bounded(&desktop.socket_name)||desktop.socket_name.contains('/')||roots.is_empty()||roots.len()>3
            || access.is_empty()||access.len()>3||access.iter().enumerate().any(|(i,a)|access[..i].contains(a))
            || digest(&"Enroll only these roots for this originating client")?!=intent.goal_sha256
            || expiry_ms==0||expiry_ms>MAX_EXPIRY_MS{return Err(ErrorCode::InvalidArgument);}
        for (i,root) in roots.iter().enumerate(){
            if !uuid(&root.root_id)||!hash(&root.identity_sha256)||!text(&root.display_path,4096)||!root.display_path.starts_with('/')
                || root.display_path=="/"||roots[..i].iter().any(|r|r.root_id==root.root_id){return Err(ErrorCode::InvalidArgument);}
        }
        let issued_ms=boottime_ms()?;let expires_ms=issued_ms.checked_add(expiry_ms).ok_or(ErrorCode::InvalidArgument)?;
        let mut proposal=FileRootsProposal{intent,desktop,roots,access,revision:self.revision.clone(),incarnation:self.incarnation,
            nonce:Uuid::new_v4(),issued_ms,expires_ms,digest:String::new()};
        proposal.digest=proposal.bound_digest()?;Ok(proposal)
    }
    /// Single-use native result, consumed only after all live identities match.
    pub fn consume_file_roots(&self,decision:NativeFileRootsDecision,subject:&Subject,current:&impl CurrentResources)->Result<()>{
        if decision.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        decision.proposal.fresh(self,subject,current)?;
        if decision.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}Ok(())
    }
}
impl FileRootsProposal {
    fn bound_digest(&self)->Result<String>{digest(&(&self.intent.subject,&self.intent.request_id,&self.intent.goal_sha256,self.intent.mode,
        &self.revision,self.incarnation,self.nonce,&self.desktop.identity_sha256,&self.desktop.socket_name,self.wire(false)))}
    pub(super) fn fresh(&self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<()>{
        subject.validate()?;
        if &self.intent.subject!=subject{return Err(ErrorCode::PermissionDenied);}
        if policy.boot_id!=subject.boot_id||policy.incarnation!=self.incarnation{return Err(ErrorCode::ApprovalExpired);}
        if policy.revision!=self.revision{return Err(ErrorCode::PolicyChanged);}
        if self.bound_digest()!=Ok(self.digest.clone()){return Err(ErrorCode::PlanChanged);}
        let now=boottime_ms()?;if now<self.issued_ms||now>=self.expires_ms||self.nonce.is_nil(){return Err(ErrorCode::ApprovalExpired);}
        if current.resolve("selected_session","graphical-session",&self.desktop.session_id)?!=self.desktop.identity_sha256{return Err(ErrorCode::TargetChanged);}
        for root in &self.roots{if current.resolve("file_root","scoped-root",&root.root_id)?!=root.identity_sha256{return Err(ErrorCode::TargetChanged);}}
        let after=boottime_ms()?;if after<self.issued_ms||after>=self.expires_ms{return Err(ErrorCode::ApprovalExpired);}Ok(())
    }
    pub(super) fn wire(&self,include_digest:bool)->Value{serde_json::json!({"schema_version":1,"kind":"file_roots",
        "digest":if include_digest{self.digest.as_str()}else{""},"uid":self.desktop.uid,"session_id":self.desktop.session_id,
        "target":"Selected file roots","profile":"Private per-user access","mode":"act","goal":"Enroll only these roots for this originating client",
        "roots":self.roots,"access":self.access,"grant_lifetime_ms":300000,"issued_ms":self.issued_ms,"expires_ms":self.expires_ms,"evidence":[]})}
    pub fn launch(self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<FileRootsConfirmation>{
        Ok(FileRootsConfirmation{native:PendingProposal::FileRoots(self).launch(policy,subject,current)?})
    }
}
impl FileRootsConfirmation {
    pub fn cancellation(&self)->Result<ReadCancellation>{self.native.cancellation()}
    pub fn withdraw(&mut self){self.native.withdraw();}
    pub fn poll(&mut self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<Option<NativeFileRootsDecision>>{
        match self.native.poll(policy,subject,current)?{
            None=>Ok(None),Some((PendingProposal::FileRoots(proposal),cancelled))=>Ok(Some(NativeFileRootsDecision{proposal,cancelled})),
            Some(_)=>{self.native.withdraw();Err(ErrorCode::PolicyChanged)},
        }
    }
}

#[cfg(test)] mod tests {
    use super::*;
    use crate::tests::{subject,policy};
    struct Resources { session:String,root:String }
    impl CurrentResources for Resources {
        fn resolve(&self,field:&str,kind:&str,_:&str)->Result<String>{
            match (field,kind){("selected_session","graphical-session")=>Ok(self.session.clone()),("file_root","scoped-root")=>Ok(self.root.clone()),_=>Err(ErrorCode::PermissionDenied)}
        }
        fn dynamic_arguments(&self,_:&str,_:&Value,_:&Scope)->Result<()>{Err(ErrorCode::UnsupportedCapability)}
    }
    fn proposal(p:&Policy,s:&Subject)->FileRootsProposal{
        p.propose_file_roots(p.authenticated_user_intent(s.clone(),Uuid::new_v4().to_string(),"Enroll only these roots for this originating client",Mode::Act).unwrap(),
            NativeDesktop{uid:s.uid,boot_id:s.boot_id.clone(),session_id:"1".into(),identity_sha256:"d".repeat(64),socket_name:"wayland-0".into()},
            vec![FileRoot{root_id:Uuid::new_v4().to_string(),display_path:"/home/tester/Documents".into(),identity_sha256:"e".repeat(64)}],vec![RootAccess::Content],90_000).unwrap()
    }
    #[test] fn native_root_decision_rechecks_subject_policy_roots_display_expiry_and_cancellation(){
        let s=subject();let p=policy(&s);let current=Resources{session:"d".repeat(64),root:"e".repeat(64)};
        for field in 0..7 {
            let mut proposal=proposal(&p,&s);let mut actual=s.clone();let mut resources=Resources{session:current.session.clone(),root:current.root.clone()};
            let cancelled=Arc::new(AtomicBool::new(false));
            match field{0=>actual.client=Client::Bus{sender:":1.999".into(),bus_id:"a".repeat(32)},1=>proposal.revision="changed".into(),
                2=>resources.root="f".repeat(64),3=>resources.session="f".repeat(64),4=>{proposal.expires_ms=proposal.issued_ms;proposal.digest=proposal.bound_digest().unwrap();},
                5=>cancelled.store(true,Ordering::Release),6=>proposal.roots[0].display_path="/home/tester/Other".into(),_=>unreachable!()}
            assert!(p.consume_file_roots(NativeFileRootsDecision{proposal,cancelled},&actual,&resources).is_err());
        }
        assert_eq!(p.consume_file_roots(NativeFileRootsDecision{proposal:proposal(&p,&s),cancelled:Arc::new(AtomicBool::new(false))},&s,&current),Ok(()));
    }
    #[test] fn root_preview_is_exact_and_duplicates_or_secret_scope_text_cannot_widen_it(){
        let s=subject();let p=policy(&s);let a=proposal(&p,&s);let b=proposal(&p,&s);assert_ne!(a.digest,b.digest);
        let wire=a.wire(true);assert_eq!(wire["kind"],"file_roots");assert_eq!(wire["access"],serde_json::json!(["content"]));
        assert_eq!(wire["grant_lifetime_ms"],300000);assert!(wire.get("approved").is_none());
        for field in 0..4{
            let mut proposal=proposal(&p,&s);
            match field{0=>proposal.roots.push(proposal.roots[0].clone()),1=>proposal.access.push(RootAccess::Content),2=>proposal.roots[0].display_path="safe\u{202e}unsafe".into(),3=>proposal.intent.mode=Mode::Ask,_=>unreachable!()}
            assert!(p.propose_file_roots(proposal.intent,proposal.desktop,proposal.roots,proposal.access,90_000).is_err());
        }
    }
}
