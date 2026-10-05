package p;
  task t2(); endtask
  function automatic logic [t2()-1:0] f(); return '1; endfunction
endpackage
module top;
  function automatic int t2(); return 8; endfunction
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
