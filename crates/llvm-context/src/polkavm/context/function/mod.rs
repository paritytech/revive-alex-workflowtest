//! The LLVM IR generator function.

pub mod declaration;
pub mod intrinsics;
pub mod llvm_runtime;
pub mod r#return;
pub mod runtime;
pub mod yul_data;

use std::collections::HashMap;

use inkwell::debug_info::AsDIScope;

use crate::optimizer::settings::size_level::SizeLevel;
use crate::optimizer::Optimizer;
use crate::polkavm::context::attribute::Attribute;
use crate::polkavm::context::pointer::Pointer;

use self::declaration::Declaration;
use self::r#return::Return;
use self::yul_data::YulData;

/// The LLVM IR generator function.
#[derive(Debug)]
pub struct Function<'ctx> {
    /// The high-level source code name.
    name: String,
    /// The LLVM function declaration.
    declaration: Declaration<'ctx>,
    /// The stack representation.
    stack: HashMap<String, Pointer<'ctx>>,
    /// The return value entity.
    r#return: Return<'ctx>,

    /// The entry block. Each LLVM IR functions must have an entry block.
    entry_block: inkwell::basic_block::BasicBlock<'ctx>,
    /// The return/leave block. LLVM IR functions may have multiple returning blocks, but it is
    /// more reasonable to have a single returning block and other high-level language returns
    /// jumping to it. This way it is easier to implement some additional checks and clean-ups
    /// before the returning.
    return_block: inkwell::basic_block::BasicBlock<'ctx>,

    /// The Yul compiler data.
    yul_data: Option<YulData>,
}

impl<'ctx> Function<'ctx> {
    /// The stack hashmap default capacity.
    const STACK_HASHMAP_INITIAL_CAPACITY: usize = 64;

    /// A shortcut constructor.
    pub fn new(
        name: String,
        declaration: Declaration<'ctx>,
        r#return: Return<'ctx>,

        entry_block: inkwell::basic_block::BasicBlock<'ctx>,
        return_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) -> Self {
        Self {
            name,
            declaration,
            stack: HashMap::with_capacity(Self::STACK_HASHMAP_INITIAL_CAPACITY),
            r#return,

            entry_block,
            return_block,

            yul_data: None,
        }
    }

    /// Returns the function name reference.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Checks whether the function is defined outside of the front-end.
    pub fn is_name_external(name: &str) -> bool {
        name.starts_with("llvm.")
            || (name.starts_with("__")
                && name != self::runtime::FUNCTION_ENTRY
                && name != self::runtime::FUNCTION_DEPLOY_CODE
                && name != self::runtime::FUNCTION_RUNTIME_CODE)
    }

    /// Returns the LLVM function declaration.
    pub fn declaration(&self) -> Declaration<'ctx> {
        self.declaration
    }

    /// Returns the debug-info scope.
    pub fn get_debug_scope(&self) -> Option<inkwell::debug_info::DIScope<'ctx>> {
        self.declaration()
            .function_value()
            .get_subprogram()
            .map(|scp| scp.as_debug_info_scope())
    }

    /// Returns the N-th parameter of the function.
    pub fn get_nth_param(&self, index: usize) -> inkwell::values::BasicValueEnum<'ctx> {
        self.declaration()
            .value
            .get_nth_param(index as u32)
            .expect("Always exists")
    }

    /// Sets the memory writer function attributes.
    pub fn set_attributes(
        llvm: &'ctx inkwell::context::Context,
        declaration: Declaration<'ctx>,
        attributes: &[Attribute],
        force: bool,
    ) {
        for attribute_kind in attributes {
            match attribute_kind {
                Attribute::Memory => unimplemented!("`memory` attributes are not implemented"),
                attribute_kind @ Attribute::AlwaysInline if force => {
                    declaration.value.remove_enum_attribute(
                        inkwell::attributes::AttributeLoc::Function,
                        Attribute::NoInline as u32,
                    );
                    declaration.value.add_attribute(
                        inkwell::attributes::AttributeLoc::Function,
                        llvm.create_enum_attribute(*attribute_kind as u32, 0),
                    );
                }
                attribute_kind @ Attribute::NoInline if force => {
                    declaration.value.remove_enum_attribute(
                        inkwell::attributes::AttributeLoc::Function,
                        Attribute::AlwaysInline as u32,
                    );
                    declaration.value.add_attribute(
                        inkwell::attributes::AttributeLoc::Function,
                        llvm.create_enum_attribute(*attribute_kind as u32, 0),
                    );
                }
                attribute_kind => declaration.value.add_attribute(
                    inkwell::attributes::AttributeLoc::Function,
                    llvm.create_enum_attribute(*attribute_kind as u32, 0),
                ),
            }
        }
    }

    /// Remove specified attributes existing on the given declaration.
    pub fn remove_attributes(declaration: Declaration, attributes: &[Attribute]) {
        for attribute in attributes.iter().filter(|attribute| {
            declaration
                .value
                .get_enum_attribute(
                    inkwell::attributes::AttributeLoc::Function,
                    **attribute as u32,
                )
                .is_some()
        }) {
            declaration.value.remove_enum_attribute(
                inkwell::attributes::AttributeLoc::Function,
                *attribute as u32,
            );
        }
    }

    /// Sets the default attributes.
    /// The attributes only affect the LLVM optimizations.
    pub fn set_default_attributes(
        llvm: &'ctx inkwell::context::Context,
        declaration: Declaration<'ctx>,
        optimizer: &Optimizer,
    ) {
        if optimizer.settings().level_middle_end_size == SizeLevel::Z {
            Self::set_attributes(
                llvm,
                declaration,
                &[Attribute::OptimizeForSize, Attribute::MinSize],
                false,
            );
        }

        Self::set_attributes(llvm, declaration, &[Attribute::NoFree], false);
    }

    /// Sets the front-end runtime attributes.
    pub fn set_frontend_runtime_attributes(
        llvm: &'ctx inkwell::context::Context,
        declaration: Declaration<'ctx>,
        optimizer: &Optimizer,
    ) {
        if optimizer.settings().level_middle_end_size == SizeLevel::Z {
            Self::set_attributes(llvm, declaration, &[Attribute::NoInline], false);
        }
    }

    /// Sets the pure function attributes.
    pub fn set_pure_function_attributes(
        llvm: &'ctx inkwell::context::Context,
        declaration: Declaration<'ctx>,
    ) {
        Self::set_attributes(
            llvm,
            declaration,
            &[
                Attribute::MustProgress,
                Attribute::NoUnwind,
                Attribute::WillReturn,
            ],
            false,
        );
    }

    /// Saves the pointer to a stack variable, returning the pointer to the shadowed variable,
    /// if it exists.
    pub fn insert_stack_pointer(
        &mut self,
        name: String,
        pointer: Pointer<'ctx>,
    ) -> Option<Pointer<'ctx>> {
        self.stack.insert(name, pointer)
    }

    /// Gets the pointer to a stack variable.
    pub fn get_stack_pointer(&self, name: &str) -> Option<Pointer<'ctx>> {
        self.stack.get(name).copied()
    }

    /// Removes the pointer to a stack variable.
    pub fn remove_stack_pointer(&mut self, name: &str) {
        self.stack.remove(name);
    }

    /// Returns the return entity representation.
    pub fn r#return(&self) -> Return<'ctx> {
        self.r#return
    }

    /// Returns the pointer to the function return value.
    /// # Panics
    /// If the pointer has not been set yet.
    pub fn return_pointer(&self) -> Option<Pointer<'ctx>> {
        self.r#return.return_pointer()
    }

    /// Returns the return data size in bytes, based on the default stack alignment.
    /// # Panics
    /// If the pointer has not been set yet.
    pub fn return_data_size(&self) -> usize {
        self.r#return.return_data_size()
    }

    /// Returns the function entry block.
    pub fn entry_block(&self) -> inkwell::basic_block::BasicBlock<'ctx> {
        self.entry_block
    }

    /// Returns the function return block.
    pub fn return_block(&self) -> inkwell::basic_block::BasicBlock<'ctx> {
        self.return_block
    }

    /// Sets the Yul data.
    pub fn set_yul_data(&mut self, data: YulData) {
        self.yul_data = Some(data);
    }

    /// Returns the Yul data reference.
    /// # Panics
    /// If the Yul data has not been initialized.
    pub fn yul(&self) -> &YulData {
        self.yul_data
            .as_ref()
            .expect("The Yul data must have been initialized")
    }

    /// Returns the Yul data mutable reference.
    /// # Panics
    /// If the Yul data has not been initialized.
    pub fn yul_mut(&mut self) -> &mut YulData {
        self.yul_data
            .as_mut()
            .expect("The Yul data must have been initialized")
    }
}
