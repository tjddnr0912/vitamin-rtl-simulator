package p;
  parameter int W = 4;
  typedef enum logic [W-1:0] { T = 4'h6, F = 4'h9 } m_t;
  function automatic bit chk();
    bit u [((T == ~F) ? 1 : -1)];
    u = '{default: 1'b0};
    return u[0];
  endfunction
  function automatic logic inv(m_t v); return ~(v inside {T, F}); endfunction
endpackage
module t;
  import p::*;
  logic o;
  assign o = inv(m_t'(4'h6));
  initial begin #1 $display("A o=%b", o); $finish; end
endmodule
