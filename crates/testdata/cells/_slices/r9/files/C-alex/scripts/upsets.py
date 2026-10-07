# upstream cocotb-tested parameter sets (hand-transcribed from tb/*/test_*.py parametrize lists)
import json,sys
repo=sys.argv[1]; out=[]
def add(m,tag,d): out.append((m,tag,d))
if repo=='verilog-axis':
    for s,mm in ((1,1),(1,4),(4,1)):
        for dw in (16,32):
            add('axis_switch',f'u{s}x{mm}d{dw}',dict(S_COUNT=s,M_COUNT=mm,DATA_WIDTH=dw,ID_ENABLE=1,S_ID_WIDTH=16,M_DEST_WIDTH=8,USER_ENABLE=1,UPDATE_TID=1))
            add('axis_ram_switch',f'u{s}x{mm}d{dw}',dict(S_COUNT=s,M_COUNT=mm,S_DATA_WIDTH=dw,M_DATA_WIDTH=40-dw if dw==32 else 32,ID_ENABLE=1,S_ID_WIDTH=16,M_DEST_WIDTH=8,USER_ENABLE=1,UPDATE_TID=1,FIFO_DEPTH=512))
    for p in (1,4):
        for rr in (0,1):
            add('axis_arb_mux',f'u{p}r{rr}',dict(S_COUNT=p,DATA_WIDTH=32,ID_ENABLE=1,S_ID_WIDTH=8,DEST_ENABLE=1,UPDATE_TID=1,ARB_TYPE_ROUND_ROBIN=rr))
        add('axis_mux',f'u{p}',dict(S_COUNT=p,DATA_WIDTH=32,ID_ENABLE=1,DEST_ENABLE=1))
        add('axis_broadcast',f'u{p}',dict(M_COUNT=p,DATA_WIDTH=32,ID_ENABLE=1,DEST_ENABLE=1))
    for td in (0,1):
        add('axis_demux',f'u4t{td}',dict(M_COUNT=4,DATA_WIDTH=32,ID_ENABLE=1,DEST_ENABLE=1,TDEST_ROUTE=td))
    for s,mm in ((8,64),(64,8),(32,16),(16,32),(8,8)):
        add('axis_adapter',f'u{s}_{mm}',dict(S_DATA_WIDTH=s,M_DATA_WIDTH=mm,ID_ENABLE=1,DEST_ENABLE=1))
        add('axis_fifo_adapter',f'u{s}_{mm}',dict(DEPTH=1024,S_DATA_WIDTH=s,M_DATA_WIDTH=mm,ID_ENABLE=1,DEST_ENABLE=1,FRAME_FIFO=1,DROP_BAD_FRAME=1))
        add('axis_async_fifo_adapter',f'u{s}_{mm}',dict(DEPTH=1024,S_DATA_WIDTH=s,M_DATA_WIDTH=mm,ID_ENABLE=1,DEST_ENABLE=1,FRAME_FIFO=1,DROP_BAD_FRAME=1))
    for ff,do,db,dwf,mwf in ((0,0,0,0,0),(1,0,0,0,0),(1,1,0,0,0),(1,1,1,0,0),(1,1,1,1,0),(1,1,1,0,1)):
        for rp,of in ((0,0),(1,0),(2,0),(1,1),(2,1)):
            for m in ('axis_fifo','axis_async_fifo'):
                add(m,f'u{ff}{do}{db}{dwf}{mwf}_{rp}{of}',dict(DEPTH=1024,DATA_WIDTH=64,ID_ENABLE=1,DEST_ENABLE=1,FRAME_FIFO=ff,DROP_OVERSIZE_FRAME=do,DROP_BAD_FRAME=db,DROP_WHEN_FULL=dwf,MARK_WHEN_FULL=mwf,RAM_PIPELINE=rp,OUTPUT_FIFO_ENABLE=of))
    for rt in (0,1,2):
        add('axis_register',f'u{rt}',dict(DATA_WIDTH=64,ID_ENABLE=1,DEST_ENABLE=1,REG_TYPE=rt))
        add('axis_pipeline_register',f'u{rt}',dict(DATA_WIDTH=64,ID_ENABLE=1,DEST_ENABLE=1,REG_TYPE=rt,LENGTH=2))
    for ln in (0,1,2,5):
        add('axis_pipeline_fifo',f'u{ln}',dict(DATA_WIDTH=64,ID_ENABLE=1,DEST_ENABLE=1,LENGTH=ln))
    for az in (0,1):
        add('axis_cobs_encode',f'u{az}',dict(APPEND_ZERO=az))
    for dw in (8,64):
        add('axis_srl_fifo',f'u{dw}',dict(DATA_WIDTH=dw,ID_ENABLE=1,DEST_ENABLE=1,DEPTH=4))
        add('axis_srl_register',f'u{dw}',dict(DATA_WIDTH=dw,ID_ENABLE=1,DEST_ENABLE=1))
        add('axis_rate_limit',f'u{dw}',dict(DATA_WIDTH=dw,ID_ENABLE=1,DEST_ENABLE=1))
        add('axis_frame_length_adjust',f'u{dw}',dict(DATA_WIDTH=dw,ID_ENABLE=1,DEST_ENABLE=1))
        add('axis_frame_length_adjust_fifo',f'u{dw}',dict(DATA_WIDTH=dw,ID_ENABLE=1,DEST_ENABLE=1))
        add('axis_tap',f'u{dw}',dict(DATA_WIDTH=dw,ID_ENABLE=1,DEST_ENABLE=1))
        add('axis_crosspoint',f'u{dw}',dict(S_COUNT=4,M_COUNT=3,DATA_WIDTH=dw,ID_ENABLE=1,DEST_ENABLE=1))
    for p in (1,2,5,8,16):
        add('arbiter',f'u{p}rr',dict(PORTS=p,ARB_TYPE_ROUND_ROBIN=1,ARB_BLOCK=1,ARB_BLOCK_ACK=1,ARB_LSB_HIGH_PRIORITY=0))
        add('priority_encoder',f'u{p}',dict(WIDTH=p,LSB_HIGH_PRIORITY=1))
if repo=='verilog-pcie':
    for w in (64,128,512):
        for m in ('pcie_us_axi_dma','pcie_us_axi_dma_rd','pcie_us_axi_dma_wr','pcie_us_axi_master','pcie_us_axi_master_rd','pcie_us_axi_master_wr',
                  'pcie_us_axil_master','pcie_us_axis_cq_demux','pcie_us_axis_rc_demux','pcie_us_if','pcie_us_if_cc','pcie_us_if_cq','pcie_us_if_rc','pcie_us_if_rq',
                  'dma_if_pcie_us','dma_if_pcie_us_rd','dma_if_pcie_us_wr'):
            add(m,f'u{w}',dict(AXIS_PCIE_DATA_WIDTH=w))
    add('pcie_us_if_rc','u256s0',dict(AXIS_PCIE_DATA_WIDTH=256,RC_STRADDLE=0))
    add('pcie_us_if','u256s0',dict(AXIS_PCIE_DATA_WIDTH=256,RC_STRADDLE=0))
    for m in ('pcie_us_if_rq','pcie_us_if_cc','pcie_us_if_cq'):
        add(m,'u512s1',dict(AXIS_PCIE_DATA_WIDTH=512,**{m[-2:].upper()+'_STRADDLE':1}))
    add('pcie_us_if','u512s1',dict(AXIS_PCIE_DATA_WIDTH=512,RQ_STRADDLE=1,CQ_STRADDLE=1,CC_STRADDLE=1))
    for w in (64,128,512):
        for m in ('dma_if_pcie','dma_if_pcie_rd','dma_if_pcie_wr','pcie_axil_master','pcie_axil_master_minimal'):
            add(m,f'u{w}',dict(TLP_DATA_WIDTH=w,TLP_SEG_COUNT=1))
    for w in (64,128):
        for m in ('pcie_axi_master','pcie_axi_master_rd','pcie_axi_master_wr'):
            add(m,f'u{w}',dict(TLP_DATA_WIDTH=w,TLP_SEG_COUNT=1))
    for w,sc in ((64,1),(128,1),(256,2),(512,2),(512,4)):
        for p in (1,4):
            add('pcie_tlp_mux',f'u{w}s{sc}p{p}',dict(PORTS=p,TLP_DATA_WIDTH=w,TLP_SEG_COUNT=sc))
            add('pcie_tlp_fifo_mux',f'u{w}s{sc}p{p}',dict(PORTS=p,TLP_DATA_WIDTH=w,IN_TLP_SEG_COUNT=sc,FIFO_DEPTH=256))
            add('pcie_tlp_demux',f'u{w}s{sc}p{p}',dict(PORTS=p,TLP_DATA_WIDTH=w,IN_TLP_SEG_COUNT=sc,FIFO_DEPTH=256))
            add('pcie_tlp_demux_bar',f'u{w}s{sc}p{p}',dict(PORTS=p,TLP_DATA_WIDTH=w,IN_TLP_SEG_COUNT=sc,FIFO_DEPTH=256))
    for w,i,o in ((64,1,1),(256,1,2),(256,2,1),(512,1,4),(512,4,1),(512,2,2),(512,4,4)):
        add('pcie_tlp_fifo',f'u{w}i{i}o{o}',dict(DEPTH=256,TLP_DATA_WIDTH=w,IN_TLP_SEG_COUNT=i,OUT_TLP_SEG_COUNT=o))
        add('pcie_tlp_fifo_raw',f'u{w}i{i}o{o}c1',dict(DEPTH=256,TLP_DATA_WIDTH=w,IN_TLP_SEG_COUNT=i,OUT_TLP_SEG_COUNT=o,CTRL_OUT_EN=1))
    for dw in (256,512):
        sc=2 if dw==512 else 1
        for m in ('pcie_s10_if','pcie_s10_if_rx','pcie_s10_if_tx'):
            add(m,f'u{dw}',dict(SEG_COUNT=sc,SEG_DATA_WIDTH=dw//sc))
    for dw in (128,256,512):
        sc=2 if dw==512 else 1
        for m in ('pcie_ptile_if','pcie_ptile_if_rx','pcie_ptile_if_tx'):
            add(m,f'u{dw}',dict(SEG_COUNT=sc,SEG_DATA_WIDTH=dw//sc))
    for rdw,adw in ((128,64),(128,128),(256,64),(256,128)):
        for m in ('dma_client_axis_sink','dma_client_axis_source'):
            add(m,f'u{rdw}_{adw}',dict(SEG_COUNT=2,SEG_DATA_WIDTH=rdw//2,AXIS_DATA_WIDTH=adw,AXIS_ID_ENABLE=1,AXIS_DEST_ENABLE=1))
    for sc in (2,4):
        for sw in (32,64):
            add('dma_psdpram',f'u{sc}_{sw}',dict(SEG_COUNT=sc,SEG_DATA_WIDTH=sw))
            add('dma_psdpram_async',f'u{sc}_{sw}',dict(SEG_COUNT=sc,SEG_DATA_WIDTH=sw))
    for aw in (64,128):
        for m in ('dma_if_axi','dma_if_axi_rd','dma_if_axi_wr'):
            add(m,f'u{aw}',dict(AXI_DATA_WIDTH=aw))
    for p in (1,4):
        for m in ('dma_if_desc_mux','pcie_axi_dma_desc_mux','dma_ram_demux','dma_ram_demux_rd','dma_ram_demux_wr','dma_if_mux','dma_if_mux_rd','dma_if_mux_wr'):
            if p==1 and m in('dma_if_desc_mux','pcie_axi_dma_desc_mux','dma_if_mux','dma_if_mux_rd','dma_if_mux_wr'): continue
            add(m,f'u{p}',dict(PORTS=p))
for m,t,d in out: print(m+'\t'+t+'\t'+json.dumps(d))
