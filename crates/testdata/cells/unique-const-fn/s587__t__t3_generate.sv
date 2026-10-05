module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [7:0] hits = 8'h00;
  logic [7:0] steps = 8'h00;
  if (f(2) == 7) begin : gi
    localparam int R = 1;
  end else begin : gi
    localparam int R = 2;
  end
  for (genvar i = 0; i < f(2) - 4; i++) begin : gf
    initial hits[i] = 1'b1;
  end
  for (genvar j = f(2) - 6; j < 6; j = j + f(2) - 5) begin : gs
    initial steps[j] = 1'b1;
  end
  case (f(2))
    7: begin : gc
      localparam int C = 1;
    end
    default: begin : gc
      localparam int C = 2;
    end
  endcase
  case (7)
    f(2): begin : gl
      localparam int K = 1;
    end
    default: begin : gl
      localparam int K = 2;
    end
  endcase
  initial begin
    #1 $display("gi=%0d hits=%b steps=%b gc=%0d gl=%0d", gi.R, hits, steps, gc.C, gl.K);
    #1 $finish;
  end
endmodule
