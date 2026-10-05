module t;
  localparam logic signed [7:0] X = -4;
  localparam N = 4;
  if ((X + {N{1'b0}}) == 8'hFC) begin : GA initial $display("R: GI=then"); end
  else begin : GB initial $display("R: GI=else"); end
  case (X + {N{1'b0}})
    8'hFC: begin : CA initial $display("R: GC=fc"); end
    default: begin : CD initial $display("R: GC=def"); end
  endcase
  initial #10 $finish;
endmodule
