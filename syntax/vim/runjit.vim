" Vim Syntax File
" Language: Runjit
" Maintainer: Arne Simon
" Latest Revision: 12.02.2018

if exists("b:current_syntax")
	finish
endif

syn keyword runjitKeywords if else loop
syn match runjitKeywords '=>'
syn match runjitNumber '[-+]\d\+\.\d*'
syn match runjitNumber '\d\+\.\d*'
syn match runjitNumber '\d\+'
syn match runjitNumber '[-+]\d\+'

syn region runjitString start='"' end='"' 
syn region runjitBlock start="{" end="}" fold transparent

syn keyword runjitTodo contained TODO FIXME XXX NOTE
syn match runjitComment "#.*$" contains=celTodo

let b:current_syntax = "runjit"

hi def link runjitNumber   Constant
hi def link runjitString   Constant
hi def link runjitKeywords Keyword
hi def link runjitBlock    Statement
hi def link runjitComment  Comment
hi def link runjitTodo     Todo
