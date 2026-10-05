`define NEVER_USED unique case (x) 0: ; endcase
module top;
  logic \unique ; logic unique_x; int unique0_y;
  initial #100 $finish;
  initial begin
    \unique = 1; unique_x = 1; unique0_y = 7;
    // unique case (x) in a comment
    /* unique0 casez */
    $display("unique case %0d %0d %0d", \unique , unique_x, unique0_y);
  end
endmodule
