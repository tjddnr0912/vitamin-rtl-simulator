module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  logic [7:0] r;
  initial begin
    r = 8'hFC;
    case (r)
      (X + {N{1'b0}}): $display("R: C=hit");
      default: $display("R: C=miss");
    endcase
  end
  initial #10 $finish;
endmodule
