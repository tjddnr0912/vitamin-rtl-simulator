`timescale 1ns/1ns
module top;
  logic [7:0] a, b, lo, hi;
  int m_star, m_gen[2], m_fork, m_rec;
  always @* begin
    case (a + b) inside [lo:hi]: m_star = 1; 8'hFF: m_star = 2; default: m_star = 0; endcase
  end
  genvar g;
  for (g = 0; g < 2; g++) begin : G
    always @* begin
      case (a) inside [g*8 : g*8 + 7]: m_gen[g] = 1; default: m_gen[g] = 0; endcase
    end
  end
  function automatic int rec(input int n);
    case (n) inside 0: return 0; [1:100]: return n + rec(n - 1); default: return -1; endcase
  endfunction
  initial begin
    $dumpfile("d11.vcd"); $dumpvars(0, top);
    a = 8'd1; b = 8'd2; lo = 8'd0; hi = 8'd2;
    #1 $display("t1 m_star=%0d gen=%0d%0d", m_star, m_gen[0], m_gen[1]);
    hi = 8'd3;
    #1 $display("t2 m_star=%0d", m_star);
    a = 8'd9;
    #1 $display("t3 m_star=%0d gen=%0d%0d", m_star, m_gen[0], m_gen[1]);
    a = 8'd250; b = 8'd5;
    #1 $display("t4 m_star=%0d", m_star);
    fork
      begin case (a) inside [8'd200:8'd255]: m_fork = 1; default: m_fork = 0; endcase end
      #1 a = 8'd0;
    join
    $display("fork m_fork=%0d", m_fork);
    m_rec = rec(4);
    $display("rec=%0d", m_rec);
    $finish;
  end
  final begin case (a) inside 0: $display("final a=0 arm"); default: $display("final default"); endcase end
  initial #1000 $finish;
endmodule
