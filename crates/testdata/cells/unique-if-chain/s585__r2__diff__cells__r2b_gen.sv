module top;
  logic a = 0, b = 0, e = 0;
  logic [1:0] r = 0;
  for (genvar g = 0; g < 2; g++) begin : gl
    always @(posedge e) begin
      unique if (a) r = 1;
      else if (b) r = 2;
      else if (g == 5) r = 3;
    end
    initial #(3 + g) begin
      priority if (a) r = 1;
      else if (b) r = 2;
    end
  end
  if (1) begin : gi
    initial #6 begin
      unique if (a) r = 1; else if (b) r = 2;
    end
    final begin
      unique if (a) $display("fa"); else if (b) $display("fb");
    end
  end
  initial begin #1 e = 1; #8 $finish; end
endmodule
