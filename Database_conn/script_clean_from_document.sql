-- Select the specified documents by their UUIDs
select d.document_uuid  from document_library.documents d
where d.document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da');

-- Select all security groups linked to the specified documents
select * from document_library.document_security_groups dsg 
inner join document_library.documents d on d.document_uuid = dsg.document_uuid 
    and d.document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da');

-- Select all document chunks linked to the specified documents
select dc.* from document_library.document_chunks dc 
inner join document_library.documents d on d.document_uuid = dc.document_uuid 
    and d.document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da');

-- Select all embeddings for the chunks of the specified documents
select demg.* from document_library.document_embeding_mistral_generic demg 
inner join document_library.document_chunks dc on demg.document_chunk_uuid = dc.document_chunk_uuid 
inner join document_library.documents d on d.document_uuid = dc.document_uuid 
    and d.document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da');

-- Delete all embeddings for the chunks of the specified documents
delete from document_library.document_embeding_mistral_generic  
where document_embeding_uuid in 
    (select document_embeding_uuid  from document_library.document_embeding_mistral_generic demg
    inner join document_library.document_chunks dc on demg.document_chunk_uuid = dc.document_chunk_uuid 
    inner join document_library.documents d on d.document_uuid = dc.document_uuid 
        and d.document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da')
    );

-- Delete all document chunks linked to the specified documents
delete from document_library.document_chunks
where document_chunk_uuid in
    (select dc.document_chunk_uuid  from document_library.document_chunks dc 
    inner join document_library.documents d on d.document_uuid = dc.document_uuid 
        and d.document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da')
    );

-- Delete all security groups linked to the specified documents
delete from  document_library.document_security_groups 
where document_security_group_uuid in
    (select dsg.document_security_group_uuid from document_library.document_security_groups dsg 
    inner join document_library.documents d on d.document_uuid = dsg.document_uuid 
        and d.document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da')
    );

-- Delete all metadata entries linked to the specified documents
delete from document_library.document_metadatas
where document_metadata_uuid in 	
    (select dm.document_metadata_uuid  from document_library.document_metadatas dm 
    inner join document_library.documents d on d.document_uuid = dm.document_uuid 
        and d.document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da')
    );

-- Delete the specified documents themselves
delete from document_library.documents 
where document_uuid in ('4dc340d5-a6ee-4d2f-82e3-8fa0d84c259e','516d56c0-92ab-4562-8909-c436f6daa00e','811f23a1-d916-40d0-aa8b-e138799f80da');