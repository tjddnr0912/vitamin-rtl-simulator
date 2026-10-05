package pa; function automatic int f(); return 3; endfunction endpackage
package pb; function automatic int f(); return 5; endfunction endpackage
module top;
  import pa::f;
  import pb::f;
  initial #1 $display("d20 f=%0d", f());
  initial #100 $finish;
endmodule
