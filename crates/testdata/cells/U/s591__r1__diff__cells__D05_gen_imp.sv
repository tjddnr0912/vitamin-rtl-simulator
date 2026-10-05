package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
module top;
  import pa::P;
  if (1) begin : g
    import pb::P;
    initial #1 $display("d05 g P=%0d", P);
  end
  initial #2 $display("d05 top P=%0d", P);
  initial #100 $finish;
endmodule
