package pa; localparam string S = "aa"; endpackage
package pb; localparam string S = "bb"; endpackage
module top;
  import pa::S;
  import pb::S;
  initial #1 $display("d18 S=%s", S);
  initial #100 $finish;
endmodule
