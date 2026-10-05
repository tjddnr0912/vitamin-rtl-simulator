module t;
  function automatic int f(int a); return a ==? 4'b1?00; endfunction
`ifndef NOI
  function automatic int g(int a); return a inside {4'b1?00}; endfunction
`endif
  initial #100 $finish;
`ifdef H1
  case (1)
    ((4'bx100 ==? 4'b1?00) || 1'b1): begin : ga initial $display("H1 item"); end
    default: begin : gb initial $display("H1 default"); end
  endcase
`endif
`ifdef H2
  case (0)
    ((4'bx100 ==? 4'b1?00) && 1'b0): begin : ga initial $display("H2 item"); end
    default: begin : gb initial $display("H2 default"); end
  endcase
`endif
`ifdef H3
  case (1)
    ((4'bx100 ==? 4'b1?00) === 1'bx): begin : ga initial $display("H3 item"); end
    default: begin : gb initial $display("H3 default"); end
  endcase
`endif
`ifdef H4
  case (1)
    ((4'bx100 ==? 4'b1?00) ? 1'b1 : 1'b1): begin : ga initial $display("H4 item"); end
    default: begin : gb initial $display("H4 default"); end
  endcase
`endif
`ifdef H5
  case (1)
    ((4'bx100 inside {4'b1?00}) || 1'b1): begin : ga initial $display("H5 item"); end
    default: begin : gb initial $display("H5 default"); end
  endcase
`endif
`ifdef H6
  case (1)
    ((4'bx100 == 4'b1100) || 1'b1): begin : ga initial $display("H6 item"); end
    default: begin : gb initial $display("H6 default"); end
  endcase
`endif
`ifdef H7
  case (0)
    ((4'bx100 ==? 4'b1?00) !== 1'bx): begin : ga initial $display("H7 item"); end
    default: begin : gb initial $display("H7 default"); end
  endcase
`endif
`ifdef H8
  case (1)
    ($isunknown(4'bx100 ==? 4'b1?00)): begin : ga initial $display("H8 item"); end
    default: begin : gb initial $display("H8 default"); end
  endcase
`endif
`ifdef FN
  localparam int PF1 = f(12);
  localparam int PF2 = f(4);
`ifndef NOI
  localparam int PG1 = g(12);
`else
  localparam int PG1 = 1;
`endif
  initial begin #2; $display("F1 %0d", PF1); $display("F2 %0d", PF2); $display("F3 %0d", PG1); $display("F4 %0d", f(12)); end
`endif
endmodule
