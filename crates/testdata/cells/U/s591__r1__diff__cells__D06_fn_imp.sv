package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::P;
  function automatic int f();
    import pb::P;
    return P;
  endfunction
  initial #1 $display("d06 f=%0d P=%0d", f(), P);
  initial #100 $finish;
endmodule
