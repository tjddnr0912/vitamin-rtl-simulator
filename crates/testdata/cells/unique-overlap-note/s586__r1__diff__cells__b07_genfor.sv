module top;
  logic [1:0] r [2] = '{2'b11, 2'b01}; int y [2];
  initial #100 $finish;
  for (genvar g = 0; g < 2; g++) begin : G
    always_comb begin
      unique casez (r[g])
        2'b?1: y[g] = 1;
        2'b1?: y[g] = 2;
        default: y[g] = 0;
      endcase
    end
  end
  initial #1 $display("y0=%0d y1=%0d", y[0], y[1]);
endmodule
