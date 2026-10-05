module t;
  logic [1:0] r; logic y, z;
  `include "inc.svh"
  initial begin
    r = 2'b11;
    chk(r, y);
    unique case (r)
      2'b11: z = 1;
      default: z = 0;
    endcase
    $display("y=%0d z=%0d", y, z);
    $finish;
  end
endmodule
