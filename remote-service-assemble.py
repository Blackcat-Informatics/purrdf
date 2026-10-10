from pathlib import Path
stage=Path(__file__).parent;root=stage.parent.parent;out=stage/'remote-native-owner-postimages'
path=Path('crates/sparql-eval/src/service.rs');s=(root/path).read_text()
s=s.replace('use crate::remote::{', 'use crate::remote::{AdmittedResolvedBindings, ServiceResolutionError, ServiceBuildError, AdmittedRemoteError, ')
def swap(start,end,text):
    global s
    a=s.index(start);b=s.index(end,a);s=s[:a]+text+'\n'+s[b:]
swap('    pub fn header(&self)', '\n}\n\n// ── Denials', '''    pub fn header(&self) -> (String,String) {
        self.header_with_memory(&mut purrdf_lex::allocation::Memory::new(&mut purrdf_lex::allocation::Resident)).expect("resident credential rendering allocation")
    }

    /// Render the same credential through its original before-growth owner.
    /// # Errors
    /// Returns physical layout, admission or allocator refusal.
    /// # Panics
    /// Panics for a Basic user id containing a colon, as `header` does.
    pub fn header_with_memory<S:purrdf_lex::allocation::Admission+?Sized>(&self,memory:&mut purrdf_lex::allocation::Memory<'_,S>) -> Result<(String,String),purrdf_lex::allocation::StorageError> {
        let name=memory.string(match self {Self::Header{name,..}=>name.as_str(),_=>"Authorization"})?;
        let value=match self {
            Self::Bearer(token)=>memory.format(&format_args!("Bearer {token}"))?,
            Self::Basic{username,password}=>{
                assert!(!username.contains(':'),"an HTTP Basic user id may not contain a colon (RFC 7617 §2): the colon is the field separator, so encoding one would move part of the user id into the password");
                let secret=memory.format(&format_args!("{username}:{password}"))?;
                let value=memory.format(&format_args!("Basic {}",purrdf_xsd::binary::Base64(secret.as_bytes())))?;
                memory.release_string(secret)?;
                value
            },
            Self::Header{value,..}=>memory.string(value)?,
        };
        Ok((name,value))
    }
''')
swap('    pub fn request_headers(&self)', '\n}\n\n/// Maps a service IRI', '''    pub fn request_headers(&self) -> Vec<(String,String)> {
        self.request_headers_with_memory(&mut purrdf_lex::allocation::Memory::new(&mut purrdf_lex::allocation::Resident)).expect("resident request header allocation")
    }

    /// Copy the configured header sequence, including its credential, through
    /// the original request owner before handing it to a transport.
    /// # Errors
    /// Returns physical layout, admission or allocator refusal.
    pub fn request_headers_with_memory<S:purrdf_lex::allocation::Admission+?Sized>(&self,memory:&mut purrdf_lex::allocation::Memory<'_,S>) -> Result<Vec<(String,String)>,purrdf_lex::allocation::StorageError> {
        let mut headers=Vec::new();
        for (name,value) in &self.headers {
            let pair=(memory.string(name)?,memory.string(value)?);
            memory.push(&mut headers,pair)?;
        }
        if self.capabilities.allows(ServiceCapability::Credentials) && let Some(credential)=&self.credential {
            let pair=credential.header_with_memory(memory)?;
            memory.push(&mut headers,pair)?;
        }
        Ok(headers)
    }
''')
a=s.index('    pub fn authorize(');b=s.index('\n}\n\n// ── The in-process',a)
body=s[a:b]
body=body.replace('    pub fn authorize(', '    fn authorization_decision(')
body=body.replace('Result<&ServiceProfile, ServiceDenial>', 'Result<&ServiceProfile, (ServiceCapability, &\'static str)>')
body=body.replace('ServiceDenial::new(\n                endpoint,\n                withheld,','(\n                withheld,')
body=body.replace('ServiceDenial::new(\n                endpoint,\n                ServiceCapability::Credentials,','(\n                ServiceCapability::Credentials,')
body=body.replace('));', '));')
# A tuple expression uses one parenthesized payload, like the removed constructor.
native='''    pub fn authorize(&self,endpoint:&str,needs:ServiceCapabilities)->Result<&ServiceProfile,ServiceDenial> {
        self.authorization_decision(endpoint,needs).map_err(|(withheld,detail)|ServiceDenial::new(endpoint,withheld,detail))
    }

    pub(crate) fn authorize_with_memory<S:purrdf_lex::allocation::Admission+?Sized>(&self,endpoint:&str,needs:ServiceCapabilities,memory:&mut purrdf_lex::allocation::Memory<'_,S>)->Result<&ServiceProfile,ServiceBuildError> {
        match self.authorization_decision(endpoint,needs) {
            Ok(profile)=>Ok(profile),
            Err((withheld,detail))=>Err(ServiceBuildError::Remote(RemoteError::Denied(ServiceDenial::new(memory.string(endpoint)?,withheld,memory.string(detail)?)))),
        }
    }

'''
s=s[:a]+native+body+s[b:]
a=s.index('    fn resolve(&self, request: ServiceRequest',s.index('impl ServiceResolver for InProcessServiceResolver'))
b=s.index('\n}\n\n// ── The router',a)
s=s[:a]+'''    fn resolve(&self,request:ServiceRequest<'_>)->Result<ResolvedBindings,RemoteError> {
        crate::remote::resident_resolution(self.resolve_admitted(request,crate::WorkspaceCapability::resident()))
    }
    fn resolve_admitted(&self,request:ServiceRequest<'_>,workspace:crate::WorkspaceCapability)->Result<AdmittedResolvedBindings,ServiceResolutionError> {
        if let Some(trip)=request.stop_trip() { return Err(crate::remote::invocation_error(trip,&workspace)?); }
        let mut frame=crate::workspace::LexicalFrame::new(&workspace);
        let result={
            let mut memory=purrdf_lex::allocation::Memory::new(&mut frame);
            if let Some(catalog)=&self.catalog { catalog.authorize_with_memory(request.endpoint,ServiceCapabilities::granting([ServiceCapability::Query]),&mut memory).map(|_|()) } else {Ok(())}
        };
        if let Err(error)=result {return Err(crate::remote::build_error(error,frame));}
        let Some(dataset)=self.datasets.get(request.endpoint) else {
            return Err(ServiceResolutionError::Invocation(AdmittedRemoteError::message(purrdf_core::SilencedKind::Transport,format_args!("no in-memory endpoint <{}>",request.endpoint),&workspace)?));
        };
        crate::remote::evaluate_in_memory_admitted(dataset,request,self,&workspace)
    }
'''+s[b:]
a=s.index('    fn resolve(&self, request: ServiceRequest',s.index('impl ServiceResolver for ServiceRouter'))
b=s.index('\n}\n\n#[cfg(test)]',a)
s=s[:a]+'''    fn resolve(&self,request:ServiceRequest<'_>)->Result<ResolvedBindings,RemoteError> {
        crate::remote::resident_resolution(self.resolve_admitted(request,crate::WorkspaceCapability::resident()))
    }
    fn resolve_admitted(&self,request:ServiceRequest<'_>,workspace:crate::WorkspaceCapability)->Result<AdmittedResolvedBindings,ServiceResolutionError> {
        if let Some(trip)=request.stop_trip() {return Err(crate::remote::invocation_error(trip,&workspace)?);}
        let resolver=self.routes.get(request.endpoint).copied().or(self.fallback);
        let Some(resolver)=resolver else {
            return AdmittedResolvedBindings::try_build(&workspace,|memory| {
                Err(ServiceBuildError::Remote(RemoteError::Denied(ServiceDenial::new(memory.string(request.endpoint)?,ServiceCapability::Query,memory.string("no resolver is routed to this service, and the router has no fallback")?))))
            });
        };
        resolver.resolve_admitted(request,workspace)
    }
'''+s[b:]
dest=out/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(s)
